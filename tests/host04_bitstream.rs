use replay_host_doctor::digest::sha256_bytes;
use replay_host_doctor::model::NvencPolicyPositionV1;
use replay_host_doctor::nvenc_bitstream::{
    MAX_NVENC_BITSTREAM_BYTES_V1, NvencBitstreamError, inspect_nvenc_bitstream,
};

#[derive(Default)]
struct BitWriter {
    bytes: Vec<u8>,
    current: u8,
    used: u8,
}

impl BitWriter {
    fn bit(&mut self, value: bool) {
        self.current = (self.current << 1) | u8::from(value);
        self.used += 1;
        if self.used == 8 {
            self.bytes.push(self.current);
            self.current = 0;
            self.used = 0;
        }
    }

    fn bits(&mut self, value: u64, count: u8) {
        for shift in (0..count).rev() {
            self.bit(((value >> shift) & 1) != 0);
        }
    }

    fn ue(&mut self, value: u32) {
        let code_num = u64::from(value) + 1;
        let width = (u64::BITS - code_num.leading_zeros()) as u8;
        for _ in 1..width {
            self.bit(false);
        }
        self.bits(code_num, width);
    }

    fn finish_rbsp(mut self) -> Vec<u8> {
        self.bit(true);
        while self.used != 0 {
            self.bit(false);
        }
        self.bytes
    }

    fn finish_zero_padded(mut self) -> Vec<u8> {
        while self.used != 0 {
            self.bit(false);
        }
        self.bytes
    }
}

fn annex_b_nal(header: &[u8], rbsp: &[u8]) -> Vec<u8> {
    let mut bytes = vec![0, 0, 0, 1];
    bytes.extend_from_slice(header);
    bytes.extend_from_slice(rbsp);
    bytes
}

fn h264_high_420_8_keyframe() -> Vec<u8> {
    let mut sps = BitWriter::default();
    sps.bits(100, 8);
    sps.bits(0, 8);
    sps.bits(51, 8);
    sps.ue(0);
    sps.ue(1);
    sps.ue(0);
    sps.ue(0);
    sps.bit(false);
    sps.bit(false);
    sps.ue(0);
    sps.ue(0);
    sps.ue(0);
    sps.ue(1);
    sps.bit(false);
    sps.ue(239);
    sps.ue(134);
    sps.bit(true);
    sps.bit(true);
    sps.bit(false);
    sps.bit(false);

    let mut stream = annex_b_nal(&[0x67], &sps.finish_rbsp());
    stream.extend(annex_b_nal(&[0x65], &[0x88, 0x80]));
    stream
}

fn hevc_keyframe(profile_idc: u8, chroma_format_idc: u32, bit_depth: u32) -> Vec<u8> {
    let mut sps = BitWriter::default();
    sps.bits(0, 4);
    sps.bits(0, 3);
    sps.bit(true);
    sps.bits(0, 2);
    sps.bit(false);
    sps.bits(u64::from(profile_idc), 5);
    sps.bits(0, 32);
    sps.bit(true);
    sps.bit(false);
    sps.bit(false);
    sps.bit(true);
    sps.bits(0, 44);
    sps.bits(153, 8);
    sps.ue(0);
    sps.ue(chroma_format_idc);
    if chroma_format_idc == 3 {
        sps.bit(false);
    }
    sps.ue(3839);
    sps.ue(2159);
    sps.bit(false);
    sps.ue(bit_depth - 8);
    sps.ue(bit_depth - 8);
    sps.ue(4);

    let mut stream = annex_b_nal(&[0x42, 0x01], &sps.finish_rbsp());
    stream.extend(annex_b_nal(&[0x26, 0x01], &[0x80]));
    stream
}

fn push_leb128(bytes: &mut Vec<u8>, mut value: usize) {
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        bytes.push(byte);
        if value == 0 {
            break;
        }
    }
}

fn av1_main_420_keyframe(high_bitdepth: bool) -> Vec<u8> {
    let mut sequence = BitWriter::default();
    sequence.bits(0, 3);
    sequence.bit(true);
    sequence.bit(true);
    sequence.bits(13, 5);
    sequence.bit(false);
    sequence.bits(11, 4);
    sequence.bits(11, 4);
    sequence.bits(3839, 12);
    sequence.bits(2159, 12);
    sequence.bit(false);
    sequence.bit(true);
    sequence.bit(true);
    sequence.bit(false);
    sequence.bit(true);
    sequence.bit(true);
    sequence.bit(high_bitdepth);
    sequence.bit(false);
    sequence.bit(false);
    sequence.bit(false);
    sequence.bits(0, 2);
    sequence.bit(false);
    sequence.bit(false);
    let sequence = sequence.finish_zero_padded();

    let mut stream = vec![0x0a];
    push_leb128(&mut stream, sequence.len());
    stream.extend(sequence);
    stream.extend([0x32, 0x01, 0x00]);
    stream
}

#[test]
fn host04_bitstream_all_seven_policy_positions_prove_from_keyframes() {
    let cases = [
        (
            NvencPolicyPositionV1::H264HighYuv420EightBit,
            h264_high_420_8_keyframe(),
        ),
        (
            NvencPolicyPositionV1::HevcMainYuv420EightBit,
            hevc_keyframe(1, 1, 8),
        ),
        (
            NvencPolicyPositionV1::HevcMain10Yuv420TenBit,
            hevc_keyframe(2, 1, 10),
        ),
        (
            NvencPolicyPositionV1::HevcFrextYuv444EightBit,
            hevc_keyframe(4, 3, 8),
        ),
        (
            NvencPolicyPositionV1::HevcFrextYuv444TenBit,
            hevc_keyframe(4, 3, 10),
        ),
        (
            NvencPolicyPositionV1::Av1MainYuv420EightBit,
            av1_main_420_keyframe(false),
        ),
        (
            NvencPolicyPositionV1::Av1MainYuv420TenBit,
            av1_main_420_keyframe(true),
        ),
    ];

    for (position, bytes) in cases {
        let proof = inspect_nvenc_bitstream(position, &bytes).expect("valid keyframe must prove");
        assert_eq!(proof.parsed_tuple, position.tuple());
        assert!(proof.keyframe);
        assert_eq!(proof.byte_len as usize, bytes.len());
        assert_eq!(proof.bitstream_sha256, sha256_bytes(&bytes));
    }
}

#[test]
fn host04_bitstream_rejects_tuple_mismatch_and_non_keyframe() {
    let bytes = h264_high_420_8_keyframe();
    assert!(matches!(
        inspect_nvenc_bitstream(NvencPolicyPositionV1::HevcMainYuv420EightBit, &bytes),
        Err(NvencBitstreamError::TupleMismatch)
    ));

    let mut non_keyframe = bytes;
    let idr_header = non_keyframe
        .iter()
        .rposition(|byte| *byte == 0x65)
        .expect("test IDR header");
    non_keyframe[idr_header] = 0x61;
    assert!(matches!(
        inspect_nvenc_bitstream(NvencPolicyPositionV1::H264HighYuv420EightBit, &non_keyframe),
        Err(NvencBitstreamError::NotKeyframe)
    ));
}

#[test]
fn host04_bitstream_parser_is_bounded_and_fail_closed() {
    let oversized = vec![0_u8; MAX_NVENC_BITSTREAM_BYTES_V1 + 1];
    assert!(matches!(
        inspect_nvenc_bitstream(NvencPolicyPositionV1::H264HighYuv420EightBit, &oversized),
        Err(NvencBitstreamError::InputTooLarge { .. })
    ));

    let malformed_av1 = [0x0a, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80];
    assert!(
        inspect_nvenc_bitstream(NvencPolicyPositionV1::Av1MainYuv420EightBit, &malformed_av1)
            .is_err()
    );

    let truncated_hevc = [0, 0, 1, 0x42, 0x01, 0x80];
    assert!(
        inspect_nvenc_bitstream(
            NvencPolicyPositionV1::HevcMainYuv420EightBit,
            &truncated_hevc
        )
        .is_err()
    );
}
