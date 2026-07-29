use crate::model::{
    CaptureAdmissionV1, CaptureBoundaryStatusV1, CaptureCleanupLedgerV1, CaptureFailureV1,
    CaptureFrameLeaseV1, CaptureFrameObservationV1, CaptureGpuIdentityV1, CaptureLifecycleEventV1,
    CapturePathEvidenceV1, CapturePixelFormatV1, CapturePrimitiveObservationV1,
    CaptureProviderKindV1, CaptureSourceEvidenceV1, CaptureSourceStatusV1, CopyEdgeKindV1,
    CopyLedgerEdgeV1, CopyLedgerV1, MAX_CAPTURE_COPY_EDGES_V1, MAX_CAPTURE_LIFECYCLE_EVENTS_V1,
    MAX_CAPTURE_PLANES_V1, NVFBC_CAPTURE_PRIMITIVES_SCHEMA_V1, NVFBC_CAPTURE_SCHEMA_V1,
    SelectedOutputV1,
};
use std::collections::HashSet;

const FULL_LIFECYCLE: [CaptureLifecycleEventV1; 9] = [
    CaptureLifecycleEventV1::LibraryLoaded,
    CaptureLifecycleEventV1::StatusQueried,
    CaptureLifecycleEventV1::ContextBound,
    CaptureLifecycleEventV1::SessionCreated,
    CaptureLifecycleEventV1::FrameGrabbed,
    CaptureLifecycleEventV1::FrameReleased,
    CaptureLifecycleEventV1::SessionDestroyed,
    CaptureLifecycleEventV1::ContextReleased,
    CaptureLifecycleEventV1::LibraryUnloaded,
];

pub trait CaptureProvider {
    fn observe(&self) -> CapturePrimitiveObservationV1;
}

#[derive(Debug, Clone)]
pub struct FixtureCaptureProvider {
    observation: CapturePrimitiveObservationV1,
}

impl FixtureCaptureProvider {
    pub fn new(observation: CapturePrimitiveObservationV1) -> Self {
        Self { observation }
    }
}

impl CaptureProvider for FixtureCaptureProvider {
    fn observe(&self) -> CapturePrimitiveObservationV1 {
        self.observation.clone()
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct LiveUnavailableCaptureProvider;

impl CaptureProvider for LiveUnavailableCaptureProvider {
    fn observe(&self) -> CapturePrimitiveObservationV1 {
        CapturePrimitiveObservationV1 {
            schema: NVFBC_CAPTURE_PRIMITIVES_SCHEMA_V1.to_owned(),
            provider: CaptureProviderKindV1::LiveUnavailable,
            source: CaptureSourceEvidenceV1 {
                status: CaptureSourceStatusV1::Unavailable,
                api_version: None,
                nvfbc_header_sha256: None,
                cuda_header_sha256: None,
            },
            binding: None,
            frame: None,
            lifecycle: Vec::new(),
            failure: Some(CaptureFailureV1::SourceUnavailable),
        }
    }
}

pub fn capture_one_frame(
    provider: &dyn CaptureProvider,
    selected: Option<&SelectedOutputV1>,
) -> CapturePathEvidenceV1 {
    evaluate_capture_observation(provider.observe(), selected)
}

pub fn evaluate_capture_observation(
    observation: CapturePrimitiveObservationV1,
    selected: Option<&SelectedOutputV1>,
) -> CapturePathEvidenceV1 {
    let cleanup = derive_cleanup(&observation.lifecycle);
    let base = |failure| CapturePathEvidenceV1 {
        schema: NVFBC_CAPTURE_SCHEMA_V1.to_owned(),
        provider: observation.provider,
        source: observation.source.clone(),
        admission: CaptureAdmissionV1::Rejected,
        binding: observation.binding.clone(),
        lease: None,
        copy_ledger: None,
        cleanup: cleanup.clone(),
        failure: Some(failure),
        nvenc_boundary: CaptureBoundaryStatusV1::Unproven,
    };

    if observation.schema != NVFBC_CAPTURE_PRIMITIVES_SCHEMA_V1
        || observation.lifecycle.len() > MAX_CAPTURE_LIFECYCLE_EVENTS_V1
        || !valid_provider_source(&observation)
    {
        return base(CaptureFailureV1::SourceMismatch);
    }
    if observation.failure.is_some() {
        return base(
            observation
                .failure
                .expect("checked capture failure must remain present"),
        );
    }
    let Some(selected) = selected else {
        return base(CaptureFailureV1::BindingMismatch);
    };
    let Some(binding) = observation.binding.as_ref() else {
        return base(CaptureFailureV1::BindingMismatch);
    };
    if !binding_matches_selected(binding, selected) {
        return base(CaptureFailureV1::BindingMismatch);
    }
    if !cleanup.complete {
        return base(CaptureFailureV1::CleanupUncertain);
    }
    let Some(frame) = observation.frame.as_ref() else {
        return base(CaptureFailureV1::InvalidFrame);
    };
    if frame.grab_status != crate::model::CaptureGrabStatusV1::Success {
        return base(grab_failure(frame.grab_status));
    }
    if !frame.is_new_frame || frame.timestamp_us == 0 {
        return base(CaptureFailureV1::StaleFrame);
    }
    if frame.width_px != selected.width_px
        || frame.height_px != selected.height_px
        || !valid_frame_geometry(frame)
    {
        return base(CaptureFailureV1::InvalidFrame);
    }
    let Some(copy_ledger) = derive_copy_ledger(frame) else {
        return base(CaptureFailureV1::InvalidCopyLedger);
    };

    CapturePathEvidenceV1 {
        schema: NVFBC_CAPTURE_SCHEMA_V1.to_owned(),
        provider: observation.provider,
        source: observation.source,
        admission: CaptureAdmissionV1::Unproven,
        binding: observation.binding,
        lease: Some(CaptureFrameLeaseV1 {
            frame_sequence: frame.frame_sequence,
            timestamp_us: frame.timestamp_us,
            new_frame: frame.is_new_frame,
            width_px: frame.width_px,
            height_px: frame.height_px,
            pixel_format: frame.pixel_format,
            pitch_bytes: frame.pitch_bytes,
            planes: frame.planes.clone(),
            required_post_processing: frame.required_post_processing,
            cursor_included: frame.cursor_included,
            cursor_mode: frame.cursor_mode,
            source_surface: frame.source_surface.clone(),
            lease_surface: frame.lease_surface.clone(),
        }),
        copy_ledger: Some(copy_ledger),
        cleanup,
        failure: None,
        nvenc_boundary: CaptureBoundaryStatusV1::Unproven,
    }
}

pub fn validate_capture_path_evidence(evidence: &CapturePathEvidenceV1) -> bool {
    if evidence.schema != NVFBC_CAPTURE_SCHEMA_V1
        || !valid_source(&evidence.source)
        || !valid_cleanup_ledger(&evidence.cleanup)
    {
        return false;
    }
    match evidence.admission {
        CaptureAdmissionV1::Unproven => {
            let (Some(binding), Some(lease), Some(ledger)) = (
                evidence.binding.as_ref(),
                evidence.lease.as_ref(),
                evidence.copy_ledger.as_ref(),
            ) else {
                return false;
            };
            evidence.failure.is_none()
                && evidence.cleanup.complete
                && valid_gpu_identity(&CaptureGpuIdentityV1 {
                    pci_bdf: binding.gpu_pci_bdf.clone(),
                    gpu_uuid: binding.gpu_uuid.clone(),
                })
                && valid_lease(lease)
                && validate_persisted_ledger(lease, ledger)
        }
        CaptureAdmissionV1::Rejected => {
            evidence.failure.is_some() && evidence.lease.is_none() && evidence.copy_ledger.is_none()
        }
    }
}

fn valid_provider_source(observation: &CapturePrimitiveObservationV1) -> bool {
    match (observation.provider, observation.source.status) {
        (CaptureProviderKindV1::Fixture, CaptureSourceStatusV1::Fixture) => {
            observation.source.api_version == Some(18)
                && observation.source.nvfbc_header_sha256.is_none()
                && observation.source.cuda_header_sha256.is_none()
        }
        (CaptureProviderKindV1::LiveUnavailable, CaptureSourceStatusV1::Unavailable) => {
            observation.source.api_version.is_none()
                && observation.source.nvfbc_header_sha256.is_none()
                && observation.source.cuda_header_sha256.is_none()
                && observation.binding.is_none()
                && observation.frame.is_none()
                && observation.lifecycle.is_empty()
                && observation.failure == Some(CaptureFailureV1::SourceUnavailable)
        }
        (CaptureProviderKindV1::SourceAuthenticated, CaptureSourceStatusV1::Authenticated) => {
            observation.source.api_version == Some(18)
                && observation.source.nvfbc_header_sha256.is_some()
                && observation.source.cuda_header_sha256.is_some()
        }
        _ => false,
    }
}

fn valid_source(source: &CaptureSourceEvidenceV1) -> bool {
    match source.status {
        CaptureSourceStatusV1::Fixture => {
            source.api_version == Some(18)
                && source.nvfbc_header_sha256.is_none()
                && source.cuda_header_sha256.is_none()
        }
        CaptureSourceStatusV1::Unavailable => {
            source.api_version.is_none()
                && source.nvfbc_header_sha256.is_none()
                && source.cuda_header_sha256.is_none()
        }
        CaptureSourceStatusV1::Authenticated => {
            source.api_version == Some(18)
                && source.nvfbc_header_sha256.is_some()
                && source.cuda_header_sha256.is_some()
        }
    }
}

fn binding_matches_selected(
    binding: &crate::model::CaptureOutputBindingV1,
    selected: &SelectedOutputV1,
) -> bool {
    binding.output_name == selected.output_name
        && binding.randr_output_xid == selected.randr_output_xid
        && binding.topology_token == selected.topology_token
        && binding.gpu_pci_bdf == selected.nvml_pci_bdf
        && binding.gpu_uuid == selected.nvml_uuid
        && selected.nvcontrol_gpu_pci_bdf == selected.nvml_pci_bdf
        && selected.nvcontrol_gpu_uuid == selected.nvml_uuid
}

fn valid_frame_geometry(frame: &CaptureFrameObservationV1) -> bool {
    if frame.frame_sequence == 0
        || frame.width_px == 0
        || frame.height_px == 0
        || frame.pitch_bytes < u32::from(frame.width_px)
        || frame.planes.is_empty()
        || frame.planes.len() > MAX_CAPTURE_PLANES_V1
        || !valid_surface_id(&frame.source_surface)
        || !valid_surface_id(&frame.lease_surface)
        || (frame.cursor_included
            != matches!(
                frame.cursor_mode,
                crate::model::CaptureCursorModeV1::NvfbcComposited
            ))
    {
        return false;
    }
    let width = u64::from(frame.width_px);
    let height = u64::from(frame.height_px);
    let stride = u64::from(frame.pitch_bytes);
    let expected_sizes: &[u64] = match frame.pixel_format {
        CapturePixelFormatV1::Nv12 => &[stride * height, stride * height / 2],
        CapturePixelFormatV1::Yuv444p => &[stride * height, stride * height, stride * height],
    };
    if frame.planes.len() != expected_sizes.len() {
        return false;
    }
    let mut offset = 0_u64;
    frame
        .planes
        .iter()
        .zip(expected_sizes)
        .enumerate()
        .all(|(index, (plane, expected_size))| {
            let valid = plane.index == index as u8
                && plane.offset_bytes == offset
                && plane.stride_bytes == frame.pitch_bytes
                && plane.size_bytes == *expected_size
                && width <= u64::from(plane.stride_bytes);
            offset = offset.saturating_add(plane.size_bytes);
            valid
        })
}

fn valid_lease(lease: &CaptureFrameLeaseV1) -> bool {
    valid_frame_geometry(&CaptureFrameObservationV1 {
        grab_status: crate::model::CaptureGrabStatusV1::Success,
        frame_sequence: lease.frame_sequence,
        timestamp_us: lease.timestamp_us,
        is_new_frame: lease.new_frame,
        width_px: lease.width_px,
        height_px: lease.height_px,
        pixel_format: lease.pixel_format,
        pitch_bytes: lease.pitch_bytes,
        planes: lease.planes.clone(),
        required_post_processing: lease.required_post_processing,
        cursor_included: lease.cursor_included,
        cursor_mode: lease.cursor_mode,
        source_surface: lease.source_surface.clone(),
        lease_surface: lease.lease_surface.clone(),
        edges: Vec::new(),
    }) && lease.new_frame
        && lease.timestamp_us > 0
}

fn valid_surface_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && !value.starts_with("0x")
        && !value.contains("pointer")
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn valid_gpu_identity(identity: &CaptureGpuIdentityV1) -> bool {
    valid_pci_bdf(&identity.pci_bdf) && valid_gpu_uuid(&identity.gpu_uuid)
}

fn valid_pci_bdf(value: &str) -> bool {
    let Some((domain, remainder)) = value.split_once(':') else {
        return false;
    };
    let Some((bus, remainder)) = remainder.split_once(':') else {
        return false;
    };
    let Some((device, function)) = remainder.split_once('.') else {
        return false;
    };
    domain.len() == 8
        && bus.len() == 2
        && device.len() == 2
        && function.len() == 1
        && [domain, bus, device, function]
            .into_iter()
            .all(|part| part.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

fn valid_gpu_uuid(value: &str) -> bool {
    value.strip_prefix("GPU-").is_some_and(|uuid| {
        let groups = uuid.split('-').collect::<Vec<_>>();
        groups.len() == 5
            && groups.iter().zip([8, 4, 4, 4, 12]).all(|(group, length)| {
                group.len() == length && group.bytes().all(|byte| byte.is_ascii_hexdigit())
            })
    })
}

fn derive_copy_ledger(frame: &CaptureFrameObservationV1) -> Option<CopyLedgerV1> {
    if frame.edges.is_empty() || frame.edges.len() > MAX_CAPTURE_COPY_EDGES_V1 {
        return None;
    }
    let mut seen = HashSet::with_capacity(frame.edges.len());
    let mut previous_surface = frame.source_surface.as_str();
    let mut zero_copy_edges = 0_u16;
    let mut device_copy_edges = 0_u16;
    let mut peer_copy_edges = 0_u16;
    let mut conversion_edges = 0_u16;
    for (index, edge) in frame.edges.iter().enumerate() {
        if edge.sequence != index as u16
            || edge.from_surface != previous_surface
            || !valid_edge(edge)
            || !seen.insert((
                edge.sequence,
                edge.from_surface.as_str(),
                edge.to_surface.as_str(),
            ))
        {
            return None;
        }
        match edge.kind {
            CopyEdgeKindV1::ZeroCopy => zero_copy_edges += 1,
            CopyEdgeKindV1::SameGpuDeviceCopy => device_copy_edges += 1,
            CopyEdgeKindV1::PeerCopy => peer_copy_edges += 1,
            CopyEdgeKindV1::Conversion => conversion_edges += 1,
            CopyEdgeKindV1::HostStaged | CopyEdgeKindV1::Unknown => return None,
        }
        previous_surface = &edge.to_surface;
    }
    if previous_surface != frame.lease_surface {
        return None;
    }
    Some(CopyLedgerV1 {
        edges: frame.edges.clone(),
        zero_copy_edges,
        device_copy_edges,
        peer_copy_edges,
        conversion_edges,
        host_staged_edges: 0,
    })
}

fn valid_edge(edge: &CopyLedgerEdgeV1) -> bool {
    if !valid_surface_id(&edge.from_surface)
        || !valid_surface_id(&edge.to_surface)
        || !valid_gpu_identity(&edge.from_gpu)
        || !valid_gpu_identity(&edge.to_gpu)
    {
        return false;
    }
    let same_gpu = edge.from_gpu == edge.to_gpu;
    match edge.kind {
        CopyEdgeKindV1::ZeroCopy => {
            same_gpu
                && edge.from_surface == edge.to_surface
                && edge.input_format == edge.output_format
                && edge.peer_access.is_none()
        }
        CopyEdgeKindV1::SameGpuDeviceCopy => {
            same_gpu
                && edge.from_surface != edge.to_surface
                && edge.input_format == edge.output_format
                && edge.peer_access.is_none()
        }
        CopyEdgeKindV1::PeerCopy => {
            !same_gpu
                && edge.from_surface != edge.to_surface
                && edge.input_format == edge.output_format
                && edge.peer_access == Some(true)
        }
        CopyEdgeKindV1::Conversion => {
            same_gpu
                && edge.from_surface != edge.to_surface
                && edge.input_format != edge.output_format
                && edge.peer_access.is_none()
        }
        CopyEdgeKindV1::HostStaged | CopyEdgeKindV1::Unknown => false,
    }
}

fn validate_persisted_ledger(lease: &CaptureFrameLeaseV1, ledger: &CopyLedgerV1) -> bool {
    if ledger.host_staged_edges != 0 {
        return false;
    }
    let frame = CaptureFrameObservationV1 {
        grab_status: crate::model::CaptureGrabStatusV1::Success,
        frame_sequence: lease.frame_sequence,
        timestamp_us: lease.timestamp_us,
        is_new_frame: lease.new_frame,
        width_px: lease.width_px,
        height_px: lease.height_px,
        pixel_format: lease.pixel_format,
        pitch_bytes: lease.pitch_bytes,
        planes: lease.planes.clone(),
        required_post_processing: lease.required_post_processing,
        cursor_included: lease.cursor_included,
        cursor_mode: lease.cursor_mode,
        source_surface: lease.source_surface.clone(),
        lease_surface: lease.lease_surface.clone(),
        edges: ledger.edges.clone(),
    };
    derive_copy_ledger(&frame).is_some_and(|derived| derived == *ledger)
}

fn derive_cleanup(events: &[CaptureLifecycleEventV1]) -> CaptureCleanupLedgerV1 {
    let split = events
        .iter()
        .position(|event| *event == CaptureLifecycleEventV1::FrameReleased)
        .unwrap_or(events.len());
    CaptureCleanupLedgerV1 {
        acquired: events[..split].to_vec(),
        released: events[split..].to_vec(),
        complete: events == FULL_LIFECYCLE,
    }
}

fn valid_cleanup_ledger(cleanup: &CaptureCleanupLedgerV1) -> bool {
    let mut combined = cleanup.acquired.clone();
    combined.extend_from_slice(&cleanup.released);
    cleanup.complete == (combined == FULL_LIFECYCLE)
        && combined.len() <= MAX_CAPTURE_LIFECYCLE_EVENTS_V1
}

fn grab_failure(status: crate::model::CaptureGrabStatusV1) -> CaptureFailureV1 {
    match status {
        crate::model::CaptureGrabStatusV1::Success => CaptureFailureV1::InvalidFrame,
        crate::model::CaptureGrabStatusV1::NoNewFrame => CaptureFailureV1::NoNewFrame,
        crate::model::CaptureGrabStatusV1::AccessDenied => CaptureFailureV1::AccessDenied,
        crate::model::CaptureGrabStatusV1::ProtectedContent => CaptureFailureV1::ProtectedContent,
        crate::model::CaptureGrabStatusV1::Busy => CaptureFailureV1::Busy,
        crate::model::CaptureGrabStatusV1::DriverError => CaptureFailureV1::InvalidFrame,
        crate::model::CaptureGrabStatusV1::ApiMismatch => CaptureFailureV1::ApiMismatch,
    }
}
