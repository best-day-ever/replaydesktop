#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct ScriptedApi {
        calls: Vec<String>,
        failure: Option<NvmlRuntimeFailureV1>,
        devices: Vec<NvmlDeviceObservationV1>,
    }

    impl ScriptedApi {
        fn passing() -> Self {
            Self {
                calls: Vec::new(),
                failure: None,
                devices: vec![NvmlDeviceObservationV1 {
                    uuid: "GPU-00000000-1111-2222-3333-444444444444".to_owned(),
                    pci_bdf: "00000000:01:00.0".to_owned(),
                }],
            }
        }

        fn failing(failure: NvmlRuntimeFailureV1) -> Self {
            Self {
                calls: Vec::new(),
                failure: Some(failure),
                devices: Self::passing().devices,
            }
        }

        fn should_fail(&self, failure: NvmlRuntimeFailureV1) -> bool {
            self.failure == Some(failure)
        }
    }

    impl NvmlApi for ScriptedApi {
        type Device = usize;

        fn initialize(&mut self) -> Result<(), NvmlRuntimeFailureV1> {
            self.calls.push("initialize".to_owned());
            if self.should_fail(NvmlRuntimeFailureV1::Initialize) {
                Err(NvmlRuntimeFailureV1::Initialize)
            } else {
                Ok(())
            }
        }

        fn driver_version(&mut self) -> Result<String, NvmlRuntimeFailureV1> {
            self.calls.push("driver-version".to_owned());
            if self.should_fail(NvmlRuntimeFailureV1::DriverVersion) {
                Err(NvmlRuntimeFailureV1::DriverVersion)
            } else {
                Ok("610.43.03".to_owned())
            }
        }

        fn device_count(&mut self) -> Result<u32, NvmlRuntimeFailureV1> {
            self.calls.push("device-count".to_owned());
            if self.should_fail(NvmlRuntimeFailureV1::DeviceCount) {
                Err(NvmlRuntimeFailureV1::DeviceCount)
            } else if self.should_fail(NvmlRuntimeFailureV1::ZeroDevices) {
                Ok(0)
            } else {
                Ok(u32::try_from(self.devices.len()).expect("fixture device count"))
            }
        }

        fn device_handle(&mut self, index: u32) -> Result<Self::Device, NvmlRuntimeFailureV1> {
            self.calls.push(format!("device-handle:{index}"));
            if self.should_fail(NvmlRuntimeFailureV1::DeviceHandle) {
                Err(NvmlRuntimeFailureV1::DeviceHandle)
            } else {
                Ok(usize::try_from(index).expect("fixture device index"))
            }
        }

        fn device_uuid(
            &mut self,
            device: Self::Device,
        ) -> Result<String, NvmlRuntimeFailureV1> {
            self.calls.push(format!("device-uuid:{device}"));
            if self.should_fail(NvmlRuntimeFailureV1::DeviceUuid) {
                Err(NvmlRuntimeFailureV1::DeviceUuid)
            } else {
                Ok(self.devices[device].uuid.clone())
            }
        }

        fn device_pci_bdf(
            &mut self,
            device: Self::Device,
        ) -> Result<String, NvmlRuntimeFailureV1> {
            self.calls.push(format!("device-pci:{device}"));
            if self.should_fail(NvmlRuntimeFailureV1::DevicePci) {
                Err(NvmlRuntimeFailureV1::DevicePci)
            } else {
                Ok(self.devices[device].pci_bdf.clone())
            }
        }

        fn shutdown(&mut self) -> Result<(), NvmlRuntimeFailureV1> {
            self.calls.push("shutdown".to_owned());
            if self.should_fail(NvmlRuntimeFailureV1::Shutdown) {
                Err(NvmlRuntimeFailureV1::Shutdown)
            } else {
                Ok(())
            }
        }
    }

    #[test]
    fn nvml_source_abi_authorized_header_matches_complete_rust_contract() {
        let source = compiled_source_metadata().expect("authorized source must be compiled");
        assert_eq!(source.identity, "nvidia-nvml-api-13");
        assert_eq!(
            source.sha256.to_string(),
            "31a26e3ce6f0b98a76cea38a3cf28aa112a20cfb37e9057786c768712d6a487f"
        );
        verify_compiled_abi().expect("C oracle and Rust declarations must agree");
        assert_eq!(NVML_SOURCE_MAP_V1.len(), 7);
        assert!(
            NVML_SOURCE_MAP_V1
                .iter()
                .all(|entry| !entry.rust_declaration.is_empty()
                    && !entry.official_declaration.is_empty()
                    && !entry.oracle_check.is_empty())
        );
    }

    #[test]
    fn nvml_runtime_complete_sequence_collects_identity_and_shutdown() {
        let mut api = ScriptedApi::passing();
        let runtime = exercise_runtime(&mut api);

        assert_eq!(runtime.failure, None);
        assert_eq!(runtime.userspace_driver_version.as_deref(), Some("610.43.03"));
        assert_eq!(runtime.devices, api.devices);
        assert!(runtime.shutdown_attempted);
        assert!(runtime.shutdown_succeeded);
        assert_eq!(
            api.calls,
            [
                "initialize",
                "driver-version",
                "device-count",
                "device-handle:0",
                "device-uuid:0",
                "device-pci:0",
                "shutdown",
            ]
        );
    }

    #[test]
    fn nvml_runtime_every_initialized_failure_shuts_down_exactly_once() {
        for failure in [
            NvmlRuntimeFailureV1::DriverVersion,
            NvmlRuntimeFailureV1::DeviceCount,
            NvmlRuntimeFailureV1::ZeroDevices,
            NvmlRuntimeFailureV1::DeviceHandle,
            NvmlRuntimeFailureV1::DeviceUuid,
            NvmlRuntimeFailureV1::DevicePci,
            NvmlRuntimeFailureV1::Shutdown,
        ] {
            let mut api = ScriptedApi::failing(failure);
            let runtime = exercise_runtime(&mut api);
            assert_eq!(runtime.failure, Some(failure), "failure {failure:?}");
            assert!(runtime.shutdown_attempted, "failure {failure:?}");
            assert_eq!(
                api.calls
                    .iter()
                    .filter(|call| call.as_str() == "shutdown")
                    .count(),
                1,
                "failure {failure:?}"
            );
        }
    }

    #[test]
    fn nvml_runtime_initialization_failure_never_calls_shutdown() {
        let mut api = ScriptedApi::failing(NvmlRuntimeFailureV1::Initialize);
        let runtime = exercise_runtime(&mut api);
        assert_eq!(
            runtime.failure,
            Some(NvmlRuntimeFailureV1::Initialize)
        );
        assert!(!runtime.shutdown_attempted);
        assert_eq!(api.calls, ["initialize"]);
    }
}
