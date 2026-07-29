use crate::digest::sha256_bytes;
use crate::model::{
    NvencChromaV1, NvencCodecV1, NvencPolicyPositionV1, NvencProfileV1, NvencStreamProofV1,
};
use std::fmt;

pub const MAX_NVENC_BITSTREAM_BYTES_V1: usize = 32 * 1024 * 1024;
const MAX_PARAMETER_SET_BYTES_V1: usize = 64 * 1024;
const MAX_STREAM_UNITS_V1: usize = 64;
const MAX_EXP_GOLOMB_ZERO_BITS_V1: u8 = 31;
const MAX_AV1_LEB128_BYTES_V1: usize = 8;
const MAX_AV1_OPERATING_POINTS_V1: u32 = 32;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum NvencBitstreamError {
    Empty,
    InputTooLarge { actual: usize, maximum: usize },
    Malformed,
    UnsupportedFraming,
    MissingSequenceHeader,
    NotKeyframe,
    TupleMismatch,
}

impl fmt::Display for NvencBitstreamError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("NVENC bitstream is empty"),
            Self::InputTooLarge { actual, maximum } => {
                write!(
                    formatter,
                    "NVENC bitstream is {actual} bytes; maximum is {maximum}"
                )
            }
            Self::Malformed => formatter.write_str("NVENC bitstream syntax is malformed"),
            Self::UnsupportedFraming => {
                formatter.write_str("NVENC bitstream framing is unsupported")
            }
            Self::MissingSequenceHeader => {
                formatter.write_str("NVENC bitstream has no leading sequence header")
            }
            Self::NotKeyframe => {
                formatter.write_str("NVENC bitstream does not begin with a keyframe")
            }
            Self::TupleMismatch => {
                formatter.write_str("NVENC bitstream identity does not match the policy tuple")
            }
        }
    }
}

impl std::error::Error for NvencBitstreamError {}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct ParsedIdentity {
    codec: NvencCodecV1,
    profile: NvencProfileV1,
    chroma: NvencChromaV1,
    bit_depth: u8,
    width_px: u32,
    height_px: u32,
    keyframe: bool,
}

pub fn inspect_nvenc_bitstream(
    position: NvencPolicyPositionV1,
    bytes: &[u8],
) -> Result<NvencStreamProofV1, NvencBitstreamError> {
    if bytes.is_empty() {
        return Err(NvencBitstreamError::Empty);
    }
    if bytes.len() > MAX_NVENC_BITSTREAM_BYTES_V1 {
        return Err(NvencBitstreamError::InputTooLarge {
            actual: bytes.len(),
            maximum: MAX_NVENC_BITSTREAM_BYTES_V1,
        });
    }

    let expected = position.tuple();
    let parsed = match parse_for_codec(expected.codec(), bytes) {
        Ok(parsed) => parsed,
        Err(primary_error) => {
            let different_codec_matches =
                [NvencCodecV1::H264, NvencCodecV1::Hevc, NvencCodecV1::Av1]
                    .into_iter()
                    .filter(|codec| *codec != expected.codec())
                    .any(|codec| parse_for_codec(codec, bytes).is_ok_and(|parsed| parsed.keyframe));
            if different_codec_matches {
                return Err(NvencBitstreamError::TupleMismatch);
            }
            return Err(primary_error);
        }
    };

    if !parsed.keyframe {
        return Err(NvencBitstreamError::NotKeyframe);
    }
    if parsed.codec != expected.codec()
        || parsed.profile != expected.profile()
        || parsed.chroma != expected.chroma()
        || parsed.bit_depth != expected.bit_depth()
        || parsed.width_px != u32::from(expected.width_px())
        || parsed.height_px != u32::from(expected.height_px())
    {
        return Err(NvencBitstreamError::TupleMismatch);
    }

    let byte_len = u32::try_from(bytes.len()).map_err(|_| NvencBitstreamError::InputTooLarge {
        actual: bytes.len(),
        maximum: MAX_NVENC_BITSTREAM_BYTES_V1,
    })?;
    Ok(NvencStreamProofV1 {
        bitstream_sha256: sha256_bytes(bytes),
        byte_len,
        keyframe: true,
        // Codec bytes prove codec/profile/chroma/depth/dimensions. The closed
        // policy position supplies its separately proven input format and rate.
        parsed_tuple: expected,
    })
}

fn parse_for_codec(
    codec: NvencCodecV1,
    bytes: &[u8],
) -> Result<ParsedIdentity, NvencBitstreamError> {
    match codec {
        NvencCodecV1::H264 => parse_h264(bytes),
        NvencCodecV1::Hevc => parse_hevc(bytes),
        NvencCodecV1::Av1 => parse_av1(bytes),
    }
}

fn parse_h264(bytes: &[u8]) -> Result<ParsedIdentity, NvencBitstreamError> {
    let units = annex_b_units(bytes)?;
    let mut sequence = None;
    let mut saw_non_keyframe_vcl = false;
    let mut keyframe = false;

    for unit in units {
        let header = *unit.first().ok_or(NvencBitstreamError::Malformed)?;
        if header & 0x80 != 0 {
            return Err(NvencBitstreamError::Malformed);
        }
        let nal_ref_idc = (header >> 5) & 0x03;
        let nal_type = header & 0x1f;
        match nal_type {
            7 => {
                if sequence.is_some() || nal_ref_idc == 0 {
                    return Err(NvencBitstreamError::Malformed);
                }
                sequence = Some(parse_h264_sps(&unit[1..])?);
            }
            5 => {
                if sequence.is_none() {
                    return Err(NvencBitstreamError::MissingSequenceHeader);
                }
                if saw_non_keyframe_vcl || nal_ref_idc == 0 {
                    return Err(NvencBitstreamError::NotKeyframe);
                }
                parse_h264_idr(&unit[1..])?;
                keyframe = true;
                break;
            }
            1..=4 => {
                if sequence.is_none() {
                    return Err(NvencBitstreamError::MissingSequenceHeader);
                }
                saw_non_keyframe_vcl = true;
            }
            _ => {}
        }
    }

    let mut identity = sequence.ok_or(NvencBitstreamError::MissingSequenceHeader)?;
    identity.keyframe = keyframe;
    Ok(identity)
}

fn parse_h264_sps(ebsp: &[u8]) -> Result<ParsedIdentity, NvencBitstreamError> {
    let rbsp = decode_ebsp(ebsp)?;
    let mut bits = BitReader::new(&rbsp);
    let profile_idc = bits.read_bits(8)?;
    let constraint_flags = bits.read_bits(8)?;
    let _level_idc = bits.read_bits(8)?;
    if profile_idc != 100 || constraint_flags & 0x03 != 0 {
        return Err(NvencBitstreamError::Malformed);
    }
    if bits.read_ue()? > 31 {
        return Err(NvencBitstreamError::Malformed);
    }
    let chroma_format_idc = bits.read_ue()?;
    if chroma_format_idc > 3 {
        return Err(NvencBitstreamError::Malformed);
    }
    let separate_colour_plane = if chroma_format_idc == 3 {
        bits.read_bit()?
    } else {
        false
    };
    if separate_colour_plane {
        return Err(NvencBitstreamError::Malformed);
    }
    let bit_depth_luma_minus8 = bits.read_ue()?;
    let bit_depth_chroma_minus8 = bits.read_ue()?;
    if bit_depth_luma_minus8 > 6 || bit_depth_luma_minus8 != bit_depth_chroma_minus8 {
        return Err(NvencBitstreamError::Malformed);
    }
    let _qpprime_y_zero_transform_bypass_flag = bits.read_bit()?;
    if bits.read_bit()? {
        return Err(NvencBitstreamError::Malformed);
    }
    if bits.read_ue()? > 12 {
        return Err(NvencBitstreamError::Malformed);
    }
    match bits.read_ue()? {
        0 => {
            if bits.read_ue()? > 12 {
                return Err(NvencBitstreamError::Malformed);
            }
        }
        1 => {
            let _delta_pic_order_always_zero_flag = bits.read_bit()?;
            let _offset_for_non_ref_pic = bits.read_se()?;
            let _offset_for_top_to_bottom_field = bits.read_se()?;
            let cycle = bits.read_ue()?;
            if cycle > 255 {
                return Err(NvencBitstreamError::Malformed);
            }
            for _ in 0..cycle {
                let _offset_for_ref_frame = bits.read_se()?;
            }
        }
        2 => {}
        _ => return Err(NvencBitstreamError::Malformed),
    }
    if bits.read_ue()? > 16 {
        return Err(NvencBitstreamError::Malformed);
    }
    let _gaps_in_frame_num_value_allowed_flag = bits.read_bit()?;
    let width_in_mbs = bits
        .read_ue()?
        .checked_add(1)
        .ok_or(NvencBitstreamError::Malformed)?;
    let height_in_map_units = bits
        .read_ue()?
        .checked_add(1)
        .ok_or(NvencBitstreamError::Malformed)?;
    let frame_mbs_only = bits.read_bit()?;
    if !frame_mbs_only {
        let _mb_adaptive_frame_field_flag = bits.read_bit()?;
    }
    let _direct_8x8_inference_flag = bits.read_bit()?;
    let cropping = bits.read_bit()?;
    let (left, right, top, bottom) = if cropping {
        (
            bits.read_ue()?,
            bits.read_ue()?,
            bits.read_ue()?,
            bits.read_ue()?,
        )
    } else {
        (0, 0, 0, 0)
    };

    let frame_factor = if frame_mbs_only { 1 } else { 2 };
    let coded_width = width_in_mbs
        .checked_mul(16)
        .ok_or(NvencBitstreamError::Malformed)?;
    let coded_height = height_in_map_units
        .checked_mul(16)
        .and_then(|height| height.checked_mul(frame_factor))
        .ok_or(NvencBitstreamError::Malformed)?;
    let (sub_width, sub_height) = match chroma_format_idc {
        0 => (1, frame_factor),
        1 => (2, 2 * frame_factor),
        2 => (2, frame_factor),
        3 => (1, frame_factor),
        _ => return Err(NvencBitstreamError::Malformed),
    };
    let crop_width = left
        .checked_add(right)
        .and_then(|value| value.checked_mul(sub_width))
        .ok_or(NvencBitstreamError::Malformed)?;
    let crop_height = top
        .checked_add(bottom)
        .and_then(|value| value.checked_mul(sub_height))
        .ok_or(NvencBitstreamError::Malformed)?;
    let width_px = coded_width
        .checked_sub(crop_width)
        .ok_or(NvencBitstreamError::Malformed)?;
    let height_px = coded_height
        .checked_sub(crop_height)
        .ok_or(NvencBitstreamError::Malformed)?;

    Ok(ParsedIdentity {
        codec: NvencCodecV1::H264,
        profile: NvencProfileV1::H264High,
        chroma: parse_chroma(chroma_format_idc)?,
        bit_depth: u8::try_from(bit_depth_luma_minus8 + 8)
            .map_err(|_| NvencBitstreamError::Malformed)?,
        width_px,
        height_px,
        keyframe: false,
    })
}

fn parse_h264_idr(ebsp: &[u8]) -> Result<(), NvencBitstreamError> {
    let rbsp = decode_ebsp(ebsp)?;
    let mut bits = BitReader::new(&rbsp);
    if bits.read_ue()? != 0 || bits.read_ue()? % 5 != 2 || bits.read_ue()? > 255 {
        return Err(NvencBitstreamError::NotKeyframe);
    }
    Ok(())
}

fn parse_hevc(bytes: &[u8]) -> Result<ParsedIdentity, NvencBitstreamError> {
    let units = annex_b_units(bytes)?;
    let mut sequence = None;
    let mut saw_non_keyframe_vcl = false;
    let mut keyframe = false;

    for unit in units {
        if unit.len() < 2 || unit[0] & 0x80 != 0 {
            return Err(NvencBitstreamError::Malformed);
        }
        let nal_type = (unit[0] >> 1) & 0x3f;
        let layer_id = ((unit[0] & 1) << 5) | (unit[1] >> 3);
        let temporal_id_plus1 = unit[1] & 0x07;
        if temporal_id_plus1 == 0 {
            return Err(NvencBitstreamError::Malformed);
        }
        match nal_type {
            33 => {
                if sequence.is_some() || layer_id != 0 {
                    return Err(NvencBitstreamError::Malformed);
                }
                sequence = Some(parse_hevc_sps(&unit[2..])?);
            }
            19 | 20 => {
                if sequence.is_none() {
                    return Err(NvencBitstreamError::MissingSequenceHeader);
                }
                if saw_non_keyframe_vcl || layer_id != 0 || temporal_id_plus1 != 1 {
                    return Err(NvencBitstreamError::NotKeyframe);
                }
                parse_hevc_idr(&unit[2..])?;
                keyframe = true;
                break;
            }
            0..=31 => {
                if sequence.is_none() {
                    return Err(NvencBitstreamError::MissingSequenceHeader);
                }
                saw_non_keyframe_vcl = true;
            }
            _ => {}
        }
    }

    let mut identity = sequence.ok_or(NvencBitstreamError::MissingSequenceHeader)?;
    identity.keyframe = keyframe;
    Ok(identity)
}

fn parse_hevc_sps(ebsp: &[u8]) -> Result<ParsedIdentity, NvencBitstreamError> {
    let rbsp = decode_ebsp(ebsp)?;
    let mut bits = BitReader::new(&rbsp);
    let _video_parameter_set_id = bits.read_bits(4)?;
    let max_sub_layers_minus1 = bits.read_bits(3)? as u8;
    if max_sub_layers_minus1 > 6 {
        return Err(NvencBitstreamError::Malformed);
    }
    let _temporal_id_nesting_flag = bits.read_bit()?;
    let profile_idc = parse_hevc_profile_tier_level(&mut bits, max_sub_layers_minus1)?;
    if bits.read_ue()? > 15 {
        return Err(NvencBitstreamError::Malformed);
    }
    let chroma_format_idc = bits.read_ue()?;
    if chroma_format_idc > 3 {
        return Err(NvencBitstreamError::Malformed);
    }
    if chroma_format_idc == 3 && bits.read_bit()? {
        return Err(NvencBitstreamError::Malformed);
    }
    let coded_width = bits.read_ue()?;
    let coded_height = bits.read_ue()?;
    if coded_width == 0 || coded_height == 0 {
        return Err(NvencBitstreamError::Malformed);
    }
    let conformance_window = bits.read_bit()?;
    let (left, right, top, bottom) = if conformance_window {
        (
            bits.read_ue()?,
            bits.read_ue()?,
            bits.read_ue()?,
            bits.read_ue()?,
        )
    } else {
        (0, 0, 0, 0)
    };
    let bit_depth_luma_minus8 = bits.read_ue()?;
    let bit_depth_chroma_minus8 = bits.read_ue()?;
    if bit_depth_luma_minus8 > 8 || bit_depth_luma_minus8 != bit_depth_chroma_minus8 {
        return Err(NvencBitstreamError::Malformed);
    }
    let _log2_max_pic_order_cnt_lsb_minus4 = bits.read_ue()?;

    let (sub_width, sub_height) = match chroma_format_idc {
        0 | 3 => (1, 1),
        1 => (2, 2),
        2 => (2, 1),
        _ => return Err(NvencBitstreamError::Malformed),
    };
    let crop_width = left
        .checked_add(right)
        .and_then(|value| value.checked_mul(sub_width))
        .ok_or(NvencBitstreamError::Malformed)?;
    let crop_height = top
        .checked_add(bottom)
        .and_then(|value| value.checked_mul(sub_height))
        .ok_or(NvencBitstreamError::Malformed)?;
    let width_px = coded_width
        .checked_sub(crop_width)
        .ok_or(NvencBitstreamError::Malformed)?;
    let height_px = coded_height
        .checked_sub(crop_height)
        .ok_or(NvencBitstreamError::Malformed)?;
    let profile = match profile_idc {
        1 => NvencProfileV1::HevcMain,
        2 => NvencProfileV1::HevcMain10,
        4 => NvencProfileV1::HevcFrext,
        _ => return Err(NvencBitstreamError::Malformed),
    };

    Ok(ParsedIdentity {
        codec: NvencCodecV1::Hevc,
        profile,
        chroma: parse_chroma(chroma_format_idc)?,
        bit_depth: u8::try_from(bit_depth_luma_minus8 + 8)
            .map_err(|_| NvencBitstreamError::Malformed)?,
        width_px,
        height_px,
        keyframe: false,
    })
}

fn parse_hevc_profile_tier_level(
    bits: &mut BitReader<'_>,
    max_sub_layers_minus1: u8,
) -> Result<u8, NvencBitstreamError> {
    let _profile_space = bits.read_bits(2)?;
    let _tier_flag = bits.read_bit()?;
    let profile_idc = bits.read_bits(5)? as u8;
    let _profile_compatibility_flags = bits.read_bits(32)?;
    let _progressive_source_flag = bits.read_bit()?;
    let _interlaced_source_flag = bits.read_bit()?;
    let _non_packed_constraint_flag = bits.read_bit()?;
    let _frame_only_constraint_flag = bits.read_bit()?;
    bits.skip_bits(44)?;
    let _level_idc = bits.read_bits(8)?;

    let mut profile_present = [false; 7];
    let mut level_present = [false; 7];
    for index in 0..usize::from(max_sub_layers_minus1) {
        profile_present[index] = bits.read_bit()?;
        level_present[index] = bits.read_bit()?;
    }
    if max_sub_layers_minus1 > 0 {
        for _ in max_sub_layers_minus1..8 {
            if bits.read_bits(2)? != 0 {
                return Err(NvencBitstreamError::Malformed);
            }
        }
    }
    for index in 0..usize::from(max_sub_layers_minus1) {
        if profile_present[index] {
            bits.skip_bits(88)?;
        }
        if level_present[index] {
            bits.skip_bits(8)?;
        }
    }
    Ok(profile_idc)
}

fn parse_hevc_idr(ebsp: &[u8]) -> Result<(), NvencBitstreamError> {
    let rbsp = decode_ebsp(ebsp)?;
    let mut bits = BitReader::new(&rbsp);
    if !bits.read_bit()? {
        return Err(NvencBitstreamError::NotKeyframe);
    }
    let _no_output_of_prior_pics_flag = bits.read_bit()?;
    if bits.read_ue()? > 63 {
        return Err(NvencBitstreamError::NotKeyframe);
    }
    Ok(())
}

fn parse_av1(bytes: &[u8]) -> Result<ParsedIdentity, NvencBitstreamError> {
    let mut cursor = 0_usize;
    let mut unit_count = 0_usize;
    let mut sequence = None;
    let mut reduced_still_picture_header = false;
    let mut keyframe = false;

    while cursor < bytes.len() {
        unit_count += 1;
        if unit_count > MAX_STREAM_UNITS_V1 {
            return Err(NvencBitstreamError::Malformed);
        }
        let header = *bytes.get(cursor).ok_or(NvencBitstreamError::Malformed)?;
        cursor += 1;
        if header & 0x81 != 0 {
            return Err(NvencBitstreamError::Malformed);
        }
        let obu_type = (header >> 3) & 0x0f;
        let extension_flag = header & 0x04 != 0;
        let has_size_field = header & 0x02 != 0;
        if !has_size_field {
            return Err(NvencBitstreamError::UnsupportedFraming);
        }
        let (temporal_id, spatial_id) = if extension_flag {
            let extension = *bytes.get(cursor).ok_or(NvencBitstreamError::Malformed)?;
            cursor += 1;
            if extension & 0x07 != 0 {
                return Err(NvencBitstreamError::Malformed);
            }
            (extension >> 5, (extension >> 3) & 0x03)
        } else {
            (0, 0)
        };
        let payload_len = read_leb128(bytes, &mut cursor)?;
        let payload_end = cursor
            .checked_add(payload_len)
            .filter(|end| *end <= bytes.len())
            .ok_or(NvencBitstreamError::Malformed)?;
        let payload = &bytes[cursor..payload_end];
        cursor = payload_end;

        match obu_type {
            1 => {
                if sequence.is_some() || keyframe || temporal_id != 0 || spatial_id != 0 {
                    return Err(NvencBitstreamError::Malformed);
                }
                let (identity, reduced) = parse_av1_sequence_header(payload)?;
                sequence = Some(identity);
                reduced_still_picture_header = reduced;
            }
            3 | 6 => {
                if sequence.is_none() {
                    return Err(NvencBitstreamError::MissingSequenceHeader);
                }
                if temporal_id != 0 || spatial_id != 0 || payload.is_empty() {
                    return Err(NvencBitstreamError::NotKeyframe);
                }
                if reduced_still_picture_header {
                    keyframe = true;
                } else {
                    let mut bits = BitReader::new(payload);
                    let show_existing_frame = bits.read_bit()?;
                    let frame_type = bits.read_bits(2)?;
                    let show_frame = bits.read_bit()?;
                    if show_existing_frame || frame_type != 0 || !show_frame {
                        return Err(NvencBitstreamError::NotKeyframe);
                    }
                    keyframe = true;
                }
                break;
            }
            _ => {}
        }
    }

    let mut identity = sequence.ok_or(NvencBitstreamError::MissingSequenceHeader)?;
    identity.keyframe = keyframe;
    Ok(identity)
}

fn parse_av1_sequence_header(
    payload: &[u8],
) -> Result<(ParsedIdentity, bool), NvencBitstreamError> {
    if payload.is_empty() || payload.len() > MAX_PARAMETER_SET_BYTES_V1 {
        return Err(NvencBitstreamError::Malformed);
    }
    let mut bits = BitReader::new(payload);
    let seq_profile = bits.read_bits(3)?;
    if seq_profile != 0 {
        return Err(NvencBitstreamError::Malformed);
    }
    let still_picture = bits.read_bit()?;
    let reduced_still_picture_header = bits.read_bit()?;
    if reduced_still_picture_header && !still_picture {
        return Err(NvencBitstreamError::Malformed);
    }

    if reduced_still_picture_header {
        let level = bits.read_bits(5)?;
        if level > 7 {
            let _seq_tier = bits.read_bit()?;
        }
    } else {
        parse_av1_operating_parameters(&mut bits)?;
    }

    let frame_width_bits = bits
        .read_bits(4)?
        .checked_add(1)
        .ok_or(NvencBitstreamError::Malformed)? as u8;
    let frame_height_bits = bits
        .read_bits(4)?
        .checked_add(1)
        .ok_or(NvencBitstreamError::Malformed)? as u8;
    let width_px = bits
        .read_bits(frame_width_bits)?
        .checked_add(1)
        .ok_or(NvencBitstreamError::Malformed)?;
    let height_px = bits
        .read_bits(frame_height_bits)?
        .checked_add(1)
        .ok_or(NvencBitstreamError::Malformed)?;

    if !reduced_still_picture_header && bits.read_bit()? {
        let _delta_frame_id_length_minus2 = bits.read_bits(4)?;
        let _additional_frame_id_length_minus1 = bits.read_bits(3)?;
    }
    let _use_128x128_superblock = bits.read_bit()?;
    let _enable_filter_intra = bits.read_bit()?;
    let _enable_intra_edge_filter = bits.read_bit()?;
    if !reduced_still_picture_header {
        let _enable_interintra_compound = bits.read_bit()?;
        let _enable_masked_compound = bits.read_bit()?;
        let _enable_warped_motion = bits.read_bit()?;
        let _enable_dual_filter = bits.read_bit()?;
        let enable_order_hint = bits.read_bit()?;
        if enable_order_hint {
            let _enable_jnt_comp = bits.read_bit()?;
            let _enable_ref_frame_mvs = bits.read_bit()?;
        }
        let choose_screen_content_tools = bits.read_bit()?;
        let force_screen_content_tools = if choose_screen_content_tools {
            2
        } else {
            u8::from(bits.read_bit()?)
        };
        if force_screen_content_tools > 0 {
            let choose_integer_mv = bits.read_bit()?;
            if !choose_integer_mv {
                let _force_integer_mv = bits.read_bit()?;
            }
        }
        if enable_order_hint {
            let _order_hint_bits_minus1 = bits.read_bits(3)?;
        }
    }
    let _enable_superres = bits.read_bit()?;
    let _enable_cdef = bits.read_bit()?;
    let _enable_restoration = bits.read_bit()?;

    let high_bitdepth = bits.read_bit()?;
    let bit_depth = if high_bitdepth { 10 } else { 8 };
    let mono_chrome = bits.read_bit()?;
    if mono_chrome {
        return Err(NvencBitstreamError::Malformed);
    }
    let color_description_present = bits.read_bit()?;
    let color_description = if color_description_present {
        Some((bits.read_bits(8)?, bits.read_bits(8)?, bits.read_bits(8)?))
    } else {
        None
    };
    if color_description == Some((1, 13, 0)) {
        return Err(NvencBitstreamError::Malformed);
    }
    let _color_range = bits.read_bit()?;
    let _chroma_sample_position = bits.read_bits(2)?;
    let _separate_uv_delta_q = bits.read_bit()?;
    let _film_grain_params_present = bits.read_bit()?;
    bits.consume_trailing_bits()?;

    Ok((
        ParsedIdentity {
            codec: NvencCodecV1::Av1,
            profile: NvencProfileV1::Av1Main,
            chroma: NvencChromaV1::Yuv420,
            bit_depth,
            width_px: u32::try_from(width_px).map_err(|_| NvencBitstreamError::Malformed)?,
            height_px: u32::try_from(height_px).map_err(|_| NvencBitstreamError::Malformed)?,
            keyframe: false,
        },
        reduced_still_picture_header,
    ))
}

fn parse_av1_operating_parameters(bits: &mut BitReader<'_>) -> Result<(), NvencBitstreamError> {
    let timing_info_present = bits.read_bit()?;
    let mut decoder_model_info = None;
    if timing_info_present {
        let _num_units_in_display_tick = bits.read_bits(32)?;
        let _time_scale = bits.read_bits(32)?;
        if bits.read_bit()? {
            let _num_ticks_per_picture_minus1 = bits.read_uvlc()?;
        }
        if bits.read_bit()? {
            let buffer_delay_length_minus1 = bits.read_bits(5)? as u8;
            let _num_units_in_decoding_tick = bits.read_bits(32)?;
            let _buffer_removal_time_length_minus1 = bits.read_bits(5)?;
            let _frame_presentation_time_length_minus1 = bits.read_bits(5)?;
            decoder_model_info = Some(buffer_delay_length_minus1 + 1);
        }
    }
    let initial_display_delay_present = bits.read_bit()?;
    let operating_points = bits
        .read_bits(5)?
        .checked_add(1)
        .ok_or(NvencBitstreamError::Malformed)?;
    if operating_points > u64::from(MAX_AV1_OPERATING_POINTS_V1) {
        return Err(NvencBitstreamError::Malformed);
    }
    for _ in 0..operating_points {
        let _operating_point_idc = bits.read_bits(12)?;
        let seq_level_idx = bits.read_bits(5)?;
        if seq_level_idx > 7 {
            let _seq_tier = bits.read_bit()?;
        }
        if let Some(buffer_delay_length) = decoder_model_info
            && bits.read_bit()?
        {
            let _decoder_buffer_delay = bits.read_bits(buffer_delay_length)?;
            let _encoder_buffer_delay = bits.read_bits(buffer_delay_length)?;
            let _low_delay_mode_flag = bits.read_bit()?;
        }
        if initial_display_delay_present && bits.read_bit()? {
            let _initial_display_delay_minus1 = bits.read_bits(4)?;
        }
    }
    Ok(())
}

fn parse_chroma(chroma_format_idc: u32) -> Result<NvencChromaV1, NvencBitstreamError> {
    match chroma_format_idc {
        1 => Ok(NvencChromaV1::Yuv420),
        3 => Ok(NvencChromaV1::Yuv444),
        _ => Err(NvencBitstreamError::Malformed),
    }
}

fn annex_b_units(bytes: &[u8]) -> Result<Vec<&[u8]>, NvencBitstreamError> {
    let (first_start, first_length) =
        find_start_code(bytes, 0).ok_or(NvencBitstreamError::UnsupportedFraming)?;
    if bytes[..first_start].iter().any(|byte| *byte != 0) {
        return Err(NvencBitstreamError::UnsupportedFraming);
    }
    let mut cursor = first_start + first_length;
    let mut units = Vec::with_capacity(8);
    loop {
        let next = find_start_code(bytes, cursor);
        let end = next.map_or(bytes.len(), |(start, _)| start);
        if end <= cursor {
            return Err(NvencBitstreamError::Malformed);
        }
        units.push(&bytes[cursor..end]);
        if units.len() > MAX_STREAM_UNITS_V1 {
            return Err(NvencBitstreamError::Malformed);
        }
        let Some((start, length)) = next else {
            break;
        };
        cursor = start
            .checked_add(length)
            .ok_or(NvencBitstreamError::Malformed)?;
        if cursor >= bytes.len() {
            return Err(NvencBitstreamError::Malformed);
        }
    }
    Ok(units)
}

fn find_start_code(bytes: &[u8], from: usize) -> Option<(usize, usize)> {
    let mut index = from;
    while index + 3 <= bytes.len() {
        if index + 4 <= bytes.len() && bytes[index..index + 4] == [0, 0, 0, 1] {
            return Some((index, 4));
        }
        if bytes[index..index + 3] == [0, 0, 1] {
            return Some((index, 3));
        }
        index += 1;
    }
    None
}

fn decode_ebsp(bytes: &[u8]) -> Result<Vec<u8>, NvencBitstreamError> {
    if bytes.is_empty() || bytes.len() > MAX_PARAMETER_SET_BYTES_V1 {
        return Err(NvencBitstreamError::Malformed);
    }
    let mut rbsp = Vec::with_capacity(bytes.len());
    let mut index = 0_usize;
    let mut zero_count = 0_u8;
    while index < bytes.len() {
        let byte = bytes[index];
        if zero_count >= 2 {
            if byte == 3 {
                let next = *bytes.get(index + 1).ok_or(NvencBitstreamError::Malformed)?;
                if next > 3 {
                    return Err(NvencBitstreamError::Malformed);
                }
                index += 1;
                zero_count = 0;
                continue;
            }
            if byte <= 2 {
                return Err(NvencBitstreamError::Malformed);
            }
        }
        rbsp.push(byte);
        zero_count = if byte == 0 {
            zero_count.saturating_add(1)
        } else {
            0
        };
        index += 1;
    }
    Ok(rbsp)
}

fn read_leb128(bytes: &[u8], cursor: &mut usize) -> Result<usize, NvencBitstreamError> {
    let mut value = 0_u64;
    for index in 0..MAX_AV1_LEB128_BYTES_V1 {
        let byte = *bytes.get(*cursor).ok_or(NvencBitstreamError::Malformed)?;
        *cursor = cursor
            .checked_add(1)
            .ok_or(NvencBitstreamError::Malformed)?;
        let shift = u32::try_from(index * 7).map_err(|_| NvencBitstreamError::Malformed)?;
        value = value
            .checked_add(u64::from(byte & 0x7f) << shift)
            .ok_or(NvencBitstreamError::Malformed)?;
        if byte & 0x80 == 0 {
            if value > u64::from(u32::MAX) {
                return Err(NvencBitstreamError::Malformed);
            }
            return usize::try_from(value).map_err(|_| NvencBitstreamError::Malformed);
        }
    }
    Err(NvencBitstreamError::Malformed)
}

struct BitReader<'a> {
    bytes: &'a [u8],
    bit_offset: usize,
}

impl<'a> BitReader<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            bit_offset: 0,
        }
    }

    fn remaining_bits(&self) -> usize {
        self.bytes
            .len()
            .saturating_mul(8)
            .saturating_sub(self.bit_offset)
    }

    fn read_bit(&mut self) -> Result<bool, NvencBitstreamError> {
        if self.bit_offset >= self.bytes.len().saturating_mul(8) {
            return Err(NvencBitstreamError::Malformed);
        }
        let byte = self.bytes[self.bit_offset / 8];
        let shift = 7 - (self.bit_offset % 8);
        self.bit_offset += 1;
        Ok((byte >> shift) & 1 != 0)
    }

    fn read_bits(&mut self, count: u8) -> Result<u64, NvencBitstreamError> {
        if count > 64 || usize::from(count) > self.remaining_bits() {
            return Err(NvencBitstreamError::Malformed);
        }
        let mut value = 0_u64;
        for _ in 0..count {
            value = (value << 1) | u64::from(self.read_bit()?);
        }
        Ok(value)
    }

    fn skip_bits(&mut self, count: u8) -> Result<(), NvencBitstreamError> {
        let count = usize::from(count);
        if count > self.remaining_bits() {
            return Err(NvencBitstreamError::Malformed);
        }
        self.bit_offset += count;
        Ok(())
    }

    fn read_ue(&mut self) -> Result<u32, NvencBitstreamError> {
        let mut leading_zeroes = 0_u8;
        while !self.read_bit()? {
            leading_zeroes += 1;
            if leading_zeroes > MAX_EXP_GOLOMB_ZERO_BITS_V1 {
                return Err(NvencBitstreamError::Malformed);
            }
        }
        let suffix = self.read_bits(leading_zeroes)?;
        let code_num = (1_u64 << leading_zeroes)
            .checked_add(suffix)
            .and_then(|value| value.checked_sub(1))
            .ok_or(NvencBitstreamError::Malformed)?;
        u32::try_from(code_num).map_err(|_| NvencBitstreamError::Malformed)
    }

    fn read_se(&mut self) -> Result<i32, NvencBitstreamError> {
        let code_num = self.read_ue()?;
        let magnitude = i64::from(code_num.div_ceil(2));
        let signed = if code_num % 2 == 0 {
            -magnitude
        } else {
            magnitude
        };
        i32::try_from(signed).map_err(|_| NvencBitstreamError::Malformed)
    }

    fn read_uvlc(&mut self) -> Result<u32, NvencBitstreamError> {
        self.read_ue()
    }

    fn consume_trailing_bits(&mut self) -> Result<(), NvencBitstreamError> {
        if !self.read_bit()? {
            return Err(NvencBitstreamError::Malformed);
        }
        while self.remaining_bits() > 0 {
            if self.read_bit()? {
                return Err(NvencBitstreamError::Malformed);
            }
        }
        Ok(())
    }
}
