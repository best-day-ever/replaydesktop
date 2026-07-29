/*
 * Project-owned ABI oracle and one-frame probe bridge for an
 * operator-supplied NVIDIA Video Codec SDK header.
 *
 * No NVIDIA declarations are copied here. Every native type, GUID, enum,
 * structure version, function signature, and layout fact consumed by the
 * bridge comes from the authenticated nvEncodeAPI.h snapshot selected by
 * build.rs.
 */
#include <stddef.h>
#include <stdint.h>
#include <string.h>

#include <nvEncodeAPI.h>

#if !defined(__GNUC__) && !defined(__clang__)
#error "NVENC ABI oracle requires a compiler with type-compatibility assertions"
#endif

/*
 * Video Codec SDK 13.1 Linux system requirements, exposed as sanitized
 * project policy facts. They are intentionally not used as substitutes for
 * the runtime API-version check below.
 */
#define REPLAY_NVENC_MIN_LINUX_DRIVER_MAJOR 610u
#define REPLAY_NVENC_MIN_CUDA_DRIVER_API 13010u
#define REPLAY_NVENC_REQUIRED_RUNTIME_API_RAW \
    ((NVENCAPI_MAJOR_VERSION << 4) | NVENCAPI_MINOR_VERSION)

enum replay_nvenc_status_class {
    REPLAY_NVENC_STATUS_SUCCESS = 0,
    REPLAY_NVENC_STATUS_UNSUPPORTED = 1,
    REPLAY_NVENC_STATUS_OPERATIONAL_REJECT = 2,
    REPLAY_NVENC_STATUS_INVALID_INPUT = 3
};

enum replay_nvenc_acquired_mask {
    REPLAY_NVENC_ACQUIRED_API = 1u << 0,
    REPLAY_NVENC_ACQUIRED_SESSION = 1u << 1,
    REPLAY_NVENC_ACQUIRED_INITIALIZED = 1u << 2,
    REPLAY_NVENC_ACQUIRED_REGISTERED = 1u << 3,
    REPLAY_NVENC_ACQUIRED_MAPPED = 1u << 4,
    REPLAY_NVENC_ACQUIRED_BITSTREAM = 1u << 5,
    REPLAY_NVENC_ACQUIRED_SUBMITTED = 1u << 6,
    REPLAY_NVENC_ACQUIRED_LOCKED = 1u << 7,
    REPLAY_NVENC_ACQUIRED_COPIED = 1u << 8
};

enum replay_nvenc_released_mask {
    REPLAY_NVENC_RELEASED_UNLOCKED = 1u << 0,
    REPLAY_NVENC_RELEASED_BITSTREAM = 1u << 1,
    REPLAY_NVENC_RELEASED_UNMAPPED = 1u << 2,
    REPLAY_NVENC_RELEASED_UNREGISTERED = 1u << 3,
    REPLAY_NVENC_RELEASED_SESSION = 1u << 4
};

enum replay_nvenc_config_mask {
    REPLAY_NVENC_CONFIG_DIMENSIONS = 1u << 0,
    REPLAY_NVENC_CONFIG_FRAMERATE = 1u << 1,
    REPLAY_NVENC_CONFIG_PRESET_P2 = 1u << 2,
    REPLAY_NVENC_CONFIG_TUNING_ULL = 1u << 3,
    REPLAY_NVENC_CONFIG_SYNCHRONOUS = 1u << 4,
    REPLAY_NVENC_CONFIG_PTD = 1u << 5,
    REPLAY_NVENC_CONFIG_FORCE_IDR_HEADERS = 1u << 6,
    REPLAY_NVENC_CONFIG_NO_B_FRAMES = 1u << 7,
    REPLAY_NVENC_CONFIG_NO_LOOKAHEAD = 1u << 8,
    REPLAY_NVENC_CONFIG_ZERO_REORDER = 1u << 9,
    REPLAY_NVENC_CONFIG_ONE_FRAME_VBV = 1u << 10
};

enum replay_nvenc_resource_mask {
    REPLAY_NVENC_RESOURCE_CUDA_CONTEXT = 1u << 0,
    REPLAY_NVENC_RESOURCE_DEVICE_POINTER = 1u << 1,
    REPLAY_NVENC_RESOURCE_ALLOCATION_BOUNDS = 1u << 2,
    REPLAY_NVENC_RESOURCE_REGISTERED = 1u << 3,
    REPLAY_NVENC_RESOURCE_MAPPED = 1u << 4,
    REPLAY_NVENC_RESOURCE_FORMAT_MATCH = 1u << 5,
    REPLAY_NVENC_RESOURCE_SUBMITTED = 1u << 6
};

struct replay_nvenc_probe_result {
    uint32_t struct_size;
    uint32_t status_class;
    uint32_t nvenc_status;
    uint32_t runtime_max_api;
    uint32_t acquired_mask;
    uint32_t released_mask;
    uint32_t config_mask;
    uint32_t resource_mask;
    uint32_t bitstream_size;
    uint32_t mapped_format;
    uint32_t picture_type;
    uint32_t cleanup_status;
};

typedef NVENCSTATUS(NVENCAPI *replay_nvenc_create_instance_fn)(
    NV_ENCODE_API_FUNCTION_LIST *);
typedef NVENCSTATUS(NVENCAPI *replay_nvenc_get_max_version_fn)(uint32_t *);

#define REPLAY_ASSERT_FUNCTION_MEMBER(member, type)                           \
    _Static_assert(__builtin_types_compatible_p(                              \
                       __typeof__(((NV_ENCODE_API_FUNCTION_LIST *)0)->member), \
                       type),                                                 \
                   "NVENC function-list member signature changed: " #member)

_Static_assert(NVENCAPI_MAJOR_VERSION == 13,
               "unsupported NVENC API major version");
_Static_assert(NVENCAPI_MINOR_VERSION == 1,
               "unsupported NVENC API minor version");
_Static_assert(NVENCAPI_VERSION == 0x0100000du,
               "unsupported NVENC API raw version");
_Static_assert(REPLAY_NVENC_REQUIRED_RUNTIME_API_RAW == 0xd1u,
               "unsupported NVENC runtime API encoding");
_Static_assert(
    __builtin_types_compatible_p(__typeof__(&NvEncodeAPICreateInstance),
                                 replay_nvenc_create_instance_fn),
    "NvEncodeAPICreateInstance signature changed");
_Static_assert(
    __builtin_types_compatible_p(__typeof__(&NvEncodeAPIGetMaxSupportedVersion),
                                 replay_nvenc_get_max_version_fn),
    "NvEncodeAPIGetMaxSupportedVersion signature changed");

REPLAY_ASSERT_FUNCTION_MEMBER(nvEncGetEncodeGUIDCount,
                              PNVENCGETENCODEGUIDCOUNT);
REPLAY_ASSERT_FUNCTION_MEMBER(nvEncGetEncodeGUIDs, PNVENCGETENCODEGUIDS);
REPLAY_ASSERT_FUNCTION_MEMBER(nvEncGetEncodeProfileGUIDCount,
                              PNVENCGETENCODEPROFILEGUIDCOUNT);
REPLAY_ASSERT_FUNCTION_MEMBER(nvEncGetEncodeProfileGUIDs,
                              PNVENCGETENCODEPROFILEGUIDS);
REPLAY_ASSERT_FUNCTION_MEMBER(nvEncGetInputFormatCount,
                              PNVENCGETINPUTFORMATCOUNT);
REPLAY_ASSERT_FUNCTION_MEMBER(nvEncGetInputFormats, PNVENCGETINPUTFORMATS);
REPLAY_ASSERT_FUNCTION_MEMBER(nvEncGetEncodeCaps, PNVENCGETENCODECAPS);
REPLAY_ASSERT_FUNCTION_MEMBER(nvEncGetEncodePresetCount,
                              PNVENCGETENCODEPRESETCOUNT);
REPLAY_ASSERT_FUNCTION_MEMBER(nvEncGetEncodePresetGUIDs,
                              PNVENCGETENCODEPRESETGUIDS);
REPLAY_ASSERT_FUNCTION_MEMBER(nvEncGetEncodePresetConfigEx,
                              PNVENCGETENCODEPRESETCONFIGEX);
REPLAY_ASSERT_FUNCTION_MEMBER(nvEncInitializeEncoder,
                              PNVENCINITIALIZEENCODER);
REPLAY_ASSERT_FUNCTION_MEMBER(nvEncCreateBitstreamBuffer,
                              PNVENCCREATEBITSTREAMBUFFER);
REPLAY_ASSERT_FUNCTION_MEMBER(nvEncDestroyBitstreamBuffer,
                              PNVENCDESTROYBITSTREAMBUFFER);
REPLAY_ASSERT_FUNCTION_MEMBER(nvEncEncodePicture, PNVENCENCODEPICTURE);
REPLAY_ASSERT_FUNCTION_MEMBER(nvEncLockBitstream, PNVENCLOCKBITSTREAM);
REPLAY_ASSERT_FUNCTION_MEMBER(nvEncUnlockBitstream, PNVENCUNLOCKBITSTREAM);
REPLAY_ASSERT_FUNCTION_MEMBER(nvEncMapInputResource,
                              PNVENCMAPINPUTRESOURCE);
REPLAY_ASSERT_FUNCTION_MEMBER(nvEncUnmapInputResource,
                              PNVENCUNMAPINPUTRESOURCE);
REPLAY_ASSERT_FUNCTION_MEMBER(nvEncDestroyEncoder, PNVENCDESTROYENCODER);
REPLAY_ASSERT_FUNCTION_MEMBER(nvEncOpenEncodeSessionEx,
                              PNVENCOPENENCODESESSIONEX);
REPLAY_ASSERT_FUNCTION_MEMBER(nvEncRegisterResource,
                              PNVENCREGISTERRESOURCE);
REPLAY_ASSERT_FUNCTION_MEMBER(nvEncUnregisterResource,
                              PNVENCUNREGISTERRESOURCE);

_Static_assert(sizeof(GUID) == 16u, "NVENC GUID layout changed");
_Static_assert(_Alignof(GUID) == 4u, "NVENC GUID alignment changed");
_Static_assert(offsetof(GUID, Data1) == 0u, "NVENC GUID Data1 moved");
_Static_assert(offsetof(GUID, Data2) == 4u, "NVENC GUID Data2 moved");
_Static_assert(offsetof(GUID, Data3) == 6u, "NVENC GUID Data3 moved");
_Static_assert(offsetof(GUID, Data4) == 8u, "NVENC GUID Data4 moved");
_Static_assert(sizeof(struct replay_nvenc_probe_result) == 48u,
               "project NVENC result layout changed");
_Static_assert(_Alignof(struct replay_nvenc_probe_result) == 4u,
               "project NVENC result alignment changed");
_Static_assert(offsetof(NV_ENCODE_API_FUNCTION_LIST, version) == 0u,
               "NVENC function-list version moved");
_Static_assert(sizeof(NV_ENCODE_API_FUNCTION_LIST) == 2552u,
               "NVENC function-list layout changed");
_Static_assert(_Alignof(NV_ENCODE_API_FUNCTION_LIST) == 8u,
               "NVENC function-list alignment changed");
_Static_assert(
    offsetof(NV_ENCODE_API_FUNCTION_LIST, nvEncGetEncodeGUIDCount) == 16u &&
        offsetof(NV_ENCODE_API_FUNCTION_LIST,
                 nvEncGetEncodeProfileGUIDCount) == 24u &&
        offsetof(NV_ENCODE_API_FUNCTION_LIST, nvEncGetEncodeProfileGUIDs) ==
            32u &&
        offsetof(NV_ENCODE_API_FUNCTION_LIST, nvEncGetEncodeGUIDs) == 40u &&
        offsetof(NV_ENCODE_API_FUNCTION_LIST, nvEncGetInputFormatCount) ==
            48u &&
        offsetof(NV_ENCODE_API_FUNCTION_LIST, nvEncGetInputFormats) == 56u &&
        offsetof(NV_ENCODE_API_FUNCTION_LIST, nvEncGetEncodeCaps) == 64u &&
        offsetof(NV_ENCODE_API_FUNCTION_LIST, nvEncGetEncodePresetCount) ==
            72u &&
        offsetof(NV_ENCODE_API_FUNCTION_LIST, nvEncGetEncodePresetGUIDs) ==
            80u,
    "NVENC capability function-list layout changed");
_Static_assert(
    offsetof(NV_ENCODE_API_FUNCTION_LIST, nvEncInitializeEncoder) == 96u &&
        offsetof(NV_ENCODE_API_FUNCTION_LIST,
                 nvEncCreateBitstreamBuffer) == 120u &&
        offsetof(NV_ENCODE_API_FUNCTION_LIST,
                 nvEncDestroyBitstreamBuffer) == 128u &&
        offsetof(NV_ENCODE_API_FUNCTION_LIST, nvEncEncodePicture) == 136u &&
        offsetof(NV_ENCODE_API_FUNCTION_LIST, nvEncLockBitstream) == 144u &&
        offsetof(NV_ENCODE_API_FUNCTION_LIST, nvEncUnlockBitstream) == 152u &&
        offsetof(NV_ENCODE_API_FUNCTION_LIST, nvEncMapInputResource) == 208u &&
        offsetof(NV_ENCODE_API_FUNCTION_LIST, nvEncUnmapInputResource) ==
            216u &&
        offsetof(NV_ENCODE_API_FUNCTION_LIST, nvEncDestroyEncoder) == 224u &&
        offsetof(NV_ENCODE_API_FUNCTION_LIST, nvEncOpenEncodeSessionEx) ==
            240u &&
        offsetof(NV_ENCODE_API_FUNCTION_LIST, nvEncRegisterResource) == 248u &&
        offsetof(NV_ENCODE_API_FUNCTION_LIST, nvEncUnregisterResource) ==
            256u &&
        offsetof(NV_ENCODE_API_FUNCTION_LIST,
                 nvEncGetEncodePresetConfigEx) == 320u,
    "NVENC execution function-list layout changed");
_Static_assert(offsetof(NV_ENC_OPEN_ENCODE_SESSION_EX_PARAMS, version) == 0u,
               "NVENC open-session version moved");
_Static_assert(sizeof(NV_ENC_OPEN_ENCODE_SESSION_EX_PARAMS) == 1552u &&
                   _Alignof(NV_ENC_OPEN_ENCODE_SESSION_EX_PARAMS) == 8u &&
                   offsetof(NV_ENC_OPEN_ENCODE_SESSION_EX_PARAMS,
                            deviceType) == 4u &&
                   offsetof(NV_ENC_OPEN_ENCODE_SESSION_EX_PARAMS, device) ==
                       8u &&
                   offsetof(NV_ENC_OPEN_ENCODE_SESSION_EX_PARAMS,
                            apiVersion) == 24u,
               "NVENC open-session layout changed");
_Static_assert(offsetof(NV_ENC_INITIALIZE_PARAMS, version) == 0u,
               "NVENC initialize version moved");
_Static_assert(
    sizeof(NV_ENC_INITIALIZE_PARAMS) == 1800u &&
        _Alignof(NV_ENC_INITIALIZE_PARAMS) == 8u &&
        offsetof(NV_ENC_INITIALIZE_PARAMS, encodeGUID) == 4u &&
        offsetof(NV_ENC_INITIALIZE_PARAMS, presetGUID) == 20u &&
        offsetof(NV_ENC_INITIALIZE_PARAMS, encodeWidth) == 36u &&
        offsetof(NV_ENC_INITIALIZE_PARAMS, encodeHeight) == 40u &&
        offsetof(NV_ENC_INITIALIZE_PARAMS, frameRateNum) == 52u &&
        offsetof(NV_ENC_INITIALIZE_PARAMS, frameRateDen) == 56u &&
        offsetof(NV_ENC_INITIALIZE_PARAMS, enableEncodeAsync) == 60u &&
        offsetof(NV_ENC_INITIALIZE_PARAMS, enablePTD) == 64u &&
        offsetof(NV_ENC_INITIALIZE_PARAMS, encodeConfig) == 88u &&
        offsetof(NV_ENC_INITIALIZE_PARAMS, maxEncodeWidth) == 96u &&
        offsetof(NV_ENC_INITIALIZE_PARAMS, maxEncodeHeight) == 100u &&
        offsetof(NV_ENC_INITIALIZE_PARAMS, tuningInfo) == 136u,
    "NVENC initialize layout changed");
_Static_assert(offsetof(NV_ENC_CONFIG, version) == 0u,
               "NVENC config version moved");
_Static_assert(sizeof(NV_ENC_CONFIG) == 3584u &&
                   _Alignof(NV_ENC_CONFIG) == 8u &&
                   offsetof(NV_ENC_CONFIG, profileGUID) == 4u &&
                   offsetof(NV_ENC_CONFIG, gopLength) == 20u &&
                   offsetof(NV_ENC_CONFIG, frameIntervalP) == 24u &&
                   offsetof(NV_ENC_CONFIG, rcParams) == 40u &&
                   offsetof(NV_ENC_CONFIG, encodeCodecConfig) == 168u,
               "NVENC config layout changed");
_Static_assert(sizeof(NV_ENC_RC_PARAMS) == 128u &&
                   _Alignof(NV_ENC_RC_PARAMS) == 4u &&
                   offsetof(NV_ENC_RC_PARAMS, version) == 0u &&
                   offsetof(NV_ENC_RC_PARAMS, averageBitRate) == 20u &&
                   offsetof(NV_ENC_RC_PARAMS, maxBitRate) == 24u &&
                   offsetof(NV_ENC_RC_PARAMS, vbvBufferSize) == 28u &&
                   offsetof(NV_ENC_RC_PARAMS, vbvInitialDelay) == 32u &&
                   offsetof(NV_ENC_RC_PARAMS, multiPass) == 100u,
               "NVENC rate-control layout changed");
_Static_assert(sizeof(NV_ENC_PRESET_CONFIG) == 5128u &&
                   _Alignof(NV_ENC_PRESET_CONFIG) == 8u &&
                   offsetof(NV_ENC_PRESET_CONFIG, version) == 0u &&
                   offsetof(NV_ENC_PRESET_CONFIG, presetCfg) == 8u,
               "NVENC preset-config layout changed");
_Static_assert(sizeof(NV_ENC_CAPS_PARAM) == 256u &&
                   _Alignof(NV_ENC_CAPS_PARAM) == 4u &&
                   offsetof(NV_ENC_CAPS_PARAM, version) == 0u &&
                   offsetof(NV_ENC_CAPS_PARAM, capsToQuery) == 4u,
               "NVENC capability-parameter layout changed");
_Static_assert(offsetof(NV_ENC_REGISTER_RESOURCE, version) == 0u,
               "NVENC register-resource version moved");
_Static_assert(
    sizeof(NV_ENC_REGISTER_RESOURCE) == 1536u &&
        _Alignof(NV_ENC_REGISTER_RESOURCE) == 8u &&
        offsetof(NV_ENC_REGISTER_RESOURCE, resourceType) == 4u &&
        offsetof(NV_ENC_REGISTER_RESOURCE, width) == 8u &&
        offsetof(NV_ENC_REGISTER_RESOURCE, height) == 12u &&
        offsetof(NV_ENC_REGISTER_RESOURCE, pitch) == 16u &&
        offsetof(NV_ENC_REGISTER_RESOURCE, resourceToRegister) == 24u &&
        offsetof(NV_ENC_REGISTER_RESOURCE, registeredResource) == 32u &&
        offsetof(NV_ENC_REGISTER_RESOURCE, bufferFormat) == 40u &&
        offsetof(NV_ENC_REGISTER_RESOURCE, bufferUsage) == 44u,
    "NVENC register-resource layout changed");
_Static_assert(offsetof(NV_ENC_MAP_INPUT_RESOURCE, version) == 0u,
               "NVENC map-resource version moved");
_Static_assert(
    sizeof(NV_ENC_MAP_INPUT_RESOURCE) == 1544u &&
        _Alignof(NV_ENC_MAP_INPUT_RESOURCE) == 8u &&
        offsetof(NV_ENC_MAP_INPUT_RESOURCE, registeredResource) == 16u &&
        offsetof(NV_ENC_MAP_INPUT_RESOURCE, mappedResource) == 24u &&
        offsetof(NV_ENC_MAP_INPUT_RESOURCE, mappedBufferFmt) == 32u,
    "NVENC map-resource layout changed");
_Static_assert(offsetof(NV_ENC_CREATE_BITSTREAM_BUFFER, version) == 0u,
               "NVENC create-bitstream version moved");
_Static_assert(
    sizeof(NV_ENC_CREATE_BITSTREAM_BUFFER) == 776u &&
        _Alignof(NV_ENC_CREATE_BITSTREAM_BUFFER) == 8u &&
        offsetof(NV_ENC_CREATE_BITSTREAM_BUFFER, bitstreamBuffer) == 16u,
    "NVENC create-bitstream layout changed");
_Static_assert(offsetof(NV_ENC_PIC_PARAMS, version) == 0u,
               "NVENC picture version moved");
_Static_assert(sizeof(NV_ENC_PIC_PARAMS) == 3360u &&
                   _Alignof(NV_ENC_PIC_PARAMS) == 8u &&
                   offsetof(NV_ENC_PIC_PARAMS, inputWidth) == 4u &&
                   offsetof(NV_ENC_PIC_PARAMS, inputHeight) == 8u &&
                   offsetof(NV_ENC_PIC_PARAMS, inputPitch) == 12u &&
                   offsetof(NV_ENC_PIC_PARAMS, encodePicFlags) == 16u &&
                   offsetof(NV_ENC_PIC_PARAMS, inputBuffer) == 40u &&
                   offsetof(NV_ENC_PIC_PARAMS, outputBitstream) == 48u &&
                   offsetof(NV_ENC_PIC_PARAMS, bufferFmt) == 64u &&
                   offsetof(NV_ENC_PIC_PARAMS, pictureStruct) == 68u &&
                   offsetof(NV_ENC_PIC_PARAMS, pictureType) == 72u,
               "NVENC picture layout changed");
_Static_assert(offsetof(NV_ENC_LOCK_BITSTREAM, version) == 0u,
               "NVENC lock-bitstream version moved");
_Static_assert(
    sizeof(NV_ENC_LOCK_BITSTREAM) == 1544u &&
        _Alignof(NV_ENC_LOCK_BITSTREAM) == 8u &&
        offsetof(NV_ENC_LOCK_BITSTREAM, outputBitstream) == 8u &&
        offsetof(NV_ENC_LOCK_BITSTREAM, bitstreamSizeInBytes) == 36u &&
        offsetof(NV_ENC_LOCK_BITSTREAM, bitstreamBufferPtr) == 56u &&
        offsetof(NV_ENC_LOCK_BITSTREAM, pictureType) == 64u,
    "NVENC lock-bitstream layout changed");

_Static_assert(NV_ENC_DEVICE_TYPE_CUDA == 1,
               "NVENC CUDA device type changed");
_Static_assert(NV_ENC_INPUT_RESOURCE_TYPE_CUDADEVICEPTR == 1,
               "NVENC CUDA resource type changed");
_Static_assert(NV_ENC_INPUT_IMAGE == 0, "NVENC input usage changed");
_Static_assert(NV_ENC_BUFFER_FORMAT_NV12 == 0x00000001,
               "NVENC NV12 format changed");
_Static_assert(NV_ENC_BUFFER_FORMAT_YUV444 == 0x00001000,
               "NVENC YUV444 format changed");
_Static_assert(NV_ENC_BUFFER_FORMAT_YUV420_10BIT == 0x00010000,
               "NVENC 10-bit YUV420 format changed");
_Static_assert(NV_ENC_BUFFER_FORMAT_YUV444_10BIT == 0x00100000,
               "NVENC 10-bit YUV444 format changed");
_Static_assert(NV_ENC_TUNING_INFO_ULTRA_LOW_LATENCY == 3,
               "NVENC ultra-low-latency tuning changed");
_Static_assert(NV_ENC_PARAMS_RC_CBR == 2, "NVENC CBR mode changed");
_Static_assert(NV_ENC_PIC_FLAG_FORCEIDR == 0x2 &&
                   NV_ENC_PIC_FLAG_OUTPUT_SPSPPS == 0x4,
               "NVENC keyframe/header flags changed");
_Static_assert(NV_ENC_PIC_STRUCT_FRAME == 1,
               "NVENC progressive picture structure changed");
_Static_assert(NV_ENC_BIT_DEPTH_8 == 8 && NV_ENC_BIT_DEPTH_10 == 10,
               "NVENC bit-depth enum changed");
_Static_assert(sizeof(NVENCSTATUS) == 4u &&
                   sizeof(NV_ENC_DEVICE_TYPE) == 4u &&
                   sizeof(NV_ENC_INPUT_RESOURCE_TYPE) == 4u &&
                   sizeof(NV_ENC_BUFFER_FORMAT) == 4u &&
                   sizeof(NV_ENC_BUFFER_USAGE) == 4u &&
                   sizeof(NV_ENC_TUNING_INFO) == 4u &&
                   sizeof(NV_ENC_BIT_DEPTH) == 4u,
               "NVENC enum representation changed");

static int replay_guid_equal(const GUID *left, const GUID *right) {
    return memcmp(left, right, sizeof(*left)) == 0;
}

static uint32_t replay_status_class_for(NVENCSTATUS status) {
    switch (status) {
    case NV_ENC_ERR_NO_ENCODE_DEVICE:
    case NV_ENC_ERR_UNSUPPORTED_DEVICE:
    case NV_ENC_ERR_UNSUPPORTED_PARAM:
    case NV_ENC_ERR_INVALID_VERSION:
    case NV_ENC_ERR_UNIMPLEMENTED:
        return REPLAY_NVENC_STATUS_UNSUPPORTED;
    default:
        return REPLAY_NVENC_STATUS_OPERATIONAL_REJECT;
    }
}

static void replay_set_failure(struct replay_nvenc_probe_result *result,
                               NVENCSTATUS status,
                               uint32_t status_class) {
    result->nvenc_status = (uint32_t)status;
    result->status_class = status_class;
}

static int replay_function_list_complete(
    const NV_ENCODE_API_FUNCTION_LIST *functions) {
    return functions->nvEncGetEncodeGUIDCount != NULL &&
           functions->nvEncGetEncodeGUIDs != NULL &&
           functions->nvEncGetEncodeProfileGUIDCount != NULL &&
           functions->nvEncGetEncodeProfileGUIDs != NULL &&
           functions->nvEncGetInputFormatCount != NULL &&
           functions->nvEncGetInputFormats != NULL &&
           functions->nvEncGetEncodeCaps != NULL &&
           functions->nvEncGetEncodePresetCount != NULL &&
           functions->nvEncGetEncodePresetGUIDs != NULL &&
           functions->nvEncGetEncodePresetConfigEx != NULL &&
           functions->nvEncInitializeEncoder != NULL &&
           functions->nvEncCreateBitstreamBuffer != NULL &&
           functions->nvEncDestroyBitstreamBuffer != NULL &&
           functions->nvEncEncodePicture != NULL &&
           functions->nvEncLockBitstream != NULL &&
           functions->nvEncUnlockBitstream != NULL &&
           functions->nvEncMapInputResource != NULL &&
           functions->nvEncUnmapInputResource != NULL &&
           functions->nvEncDestroyEncoder != NULL &&
           functions->nvEncOpenEncodeSessionEx != NULL &&
           functions->nvEncRegisterResource != NULL &&
           functions->nvEncUnregisterResource != NULL;
}

static int replay_guid_supported(PNVENCGETENCODEGUIDCOUNT get_count,
                                 PNVENCGETENCODEGUIDS get_values,
                                 void *encoder,
                                 const GUID *wanted,
                                 NVENCSTATUS *status_out) {
    GUID values[64];
    uint32_t count = 0;
    uint32_t written = 0;
    NVENCSTATUS status = get_count(encoder, &count);
    if (status != NV_ENC_SUCCESS) {
        *status_out = status;
        return -1;
    }
    if (count == 0) {
        *status_out = NV_ENC_ERR_UNSUPPORTED_PARAM;
        return 0;
    }
    if (count > (uint32_t)(sizeof(values) / sizeof(values[0]))) {
        *status_out = NV_ENC_ERR_NOT_ENOUGH_BUFFER;
        return -1;
    }
    memset(values, 0, sizeof(values));
    status = get_values(encoder, values, count, &written);
    if (status != NV_ENC_SUCCESS) {
        *status_out = status;
        return -1;
    }
    if (written > count) {
        *status_out = NV_ENC_ERR_GENERIC;
        return -1;
    }
    for (uint32_t index = 0; index < written; ++index) {
        if (replay_guid_equal(&values[index], wanted)) {
            *status_out = NV_ENC_SUCCESS;
            return 1;
        }
    }
    *status_out = NV_ENC_ERR_UNSUPPORTED_PARAM;
    return 0;
}

static int replay_profile_supported(
    const NV_ENCODE_API_FUNCTION_LIST *functions,
    void *encoder,
    GUID codec,
    const GUID *wanted,
    NVENCSTATUS *status_out) {
    GUID values[64];
    uint32_t count = 0;
    uint32_t written = 0;
    NVENCSTATUS status =
        functions->nvEncGetEncodeProfileGUIDCount(encoder, codec, &count);
    if (status != NV_ENC_SUCCESS) {
        *status_out = status;
        return -1;
    }
    if (count == 0) {
        *status_out = NV_ENC_ERR_UNSUPPORTED_PARAM;
        return 0;
    }
    if (count > (uint32_t)(sizeof(values) / sizeof(values[0]))) {
        *status_out = NV_ENC_ERR_NOT_ENOUGH_BUFFER;
        return -1;
    }
    memset(values, 0, sizeof(values));
    status = functions->nvEncGetEncodeProfileGUIDs(encoder, codec, values,
                                                    count, &written);
    if (status != NV_ENC_SUCCESS) {
        *status_out = status;
        return -1;
    }
    if (written > count) {
        *status_out = NV_ENC_ERR_GENERIC;
        return -1;
    }
    for (uint32_t index = 0; index < written; ++index) {
        if (replay_guid_equal(&values[index], wanted)) {
            *status_out = NV_ENC_SUCCESS;
            return 1;
        }
    }
    *status_out = NV_ENC_ERR_UNSUPPORTED_PARAM;
    return 0;
}

static int replay_format_supported(
    const NV_ENCODE_API_FUNCTION_LIST *functions,
    void *encoder,
    GUID codec,
    NV_ENC_BUFFER_FORMAT wanted,
    NVENCSTATUS *status_out) {
    NV_ENC_BUFFER_FORMAT values[64];
    uint32_t count = 0;
    uint32_t written = 0;
    NVENCSTATUS status =
        functions->nvEncGetInputFormatCount(encoder, codec, &count);
    if (status != NV_ENC_SUCCESS) {
        *status_out = status;
        return -1;
    }
    if (count == 0) {
        *status_out = NV_ENC_ERR_UNSUPPORTED_PARAM;
        return 0;
    }
    if (count > (uint32_t)(sizeof(values) / sizeof(values[0]))) {
        *status_out = NV_ENC_ERR_NOT_ENOUGH_BUFFER;
        return -1;
    }
    memset(values, 0, sizeof(values));
    status = functions->nvEncGetInputFormats(encoder, codec, values, count,
                                             &written);
    if (status != NV_ENC_SUCCESS) {
        *status_out = status;
        return -1;
    }
    if (written > count) {
        *status_out = NV_ENC_ERR_GENERIC;
        return -1;
    }
    for (uint32_t index = 0; index < written; ++index) {
        if (values[index] == wanted) {
            *status_out = NV_ENC_SUCCESS;
            return 1;
        }
    }
    *status_out = NV_ENC_ERR_UNSUPPORTED_PARAM;
    return 0;
}

static int replay_preset_supported(
    const NV_ENCODE_API_FUNCTION_LIST *functions,
    void *encoder,
    GUID codec,
    const GUID *wanted,
    NVENCSTATUS *status_out) {
    GUID values[64];
    uint32_t count = 0;
    uint32_t written = 0;
    NVENCSTATUS status =
        functions->nvEncGetEncodePresetCount(encoder, codec, &count);
    if (status != NV_ENC_SUCCESS) {
        *status_out = status;
        return -1;
    }
    if (count == 0) {
        *status_out = NV_ENC_ERR_UNSUPPORTED_PARAM;
        return 0;
    }
    if (count > (uint32_t)(sizeof(values) / sizeof(values[0]))) {
        *status_out = NV_ENC_ERR_NOT_ENOUGH_BUFFER;
        return -1;
    }
    memset(values, 0, sizeof(values));
    status = functions->nvEncGetEncodePresetGUIDs(encoder, codec, values,
                                                  count, &written);
    if (status != NV_ENC_SUCCESS) {
        *status_out = status;
        return -1;
    }
    if (written > count) {
        *status_out = NV_ENC_ERR_GENERIC;
        return -1;
    }
    for (uint32_t index = 0; index < written; ++index) {
        if (replay_guid_equal(&values[index], wanted)) {
            *status_out = NV_ENC_SUCCESS;
            return 1;
        }
    }
    *status_out = NV_ENC_ERR_UNSUPPORTED_PARAM;
    return 0;
}

static int replay_capability(const NV_ENCODE_API_FUNCTION_LIST *functions,
                             void *encoder,
                             GUID codec,
                             NV_ENC_CAPS capability,
                             int *value_out,
                             NVENCSTATUS *status_out) {
    NV_ENC_CAPS_PARAM params;
    memset(&params, 0, sizeof(params));
    params.version = NV_ENC_CAPS_PARAM_VER;
    params.capsToQuery = capability;
    *status_out =
        functions->nvEncGetEncodeCaps(encoder, codec, &params, value_out);
    return *status_out == NV_ENC_SUCCESS;
}

static int replay_policy(uint32_t position,
                         GUID *codec,
                         GUID *profile,
                         NV_ENC_BUFFER_FORMAT *format,
                         uint32_t *chroma,
                         NV_ENC_BIT_DEPTH *bit_depth) {
    switch (position) {
    case 0:
        *codec = NV_ENC_CODEC_H264_GUID;
        *profile = NV_ENC_H264_PROFILE_HIGH_GUID;
        *format = NV_ENC_BUFFER_FORMAT_NV12;
        *chroma = 1;
        *bit_depth = NV_ENC_BIT_DEPTH_8;
        return 1;
    case 1:
        *codec = NV_ENC_CODEC_HEVC_GUID;
        *profile = NV_ENC_HEVC_PROFILE_MAIN_GUID;
        *format = NV_ENC_BUFFER_FORMAT_NV12;
        *chroma = 1;
        *bit_depth = NV_ENC_BIT_DEPTH_8;
        return 1;
    case 2:
        *codec = NV_ENC_CODEC_HEVC_GUID;
        *profile = NV_ENC_HEVC_PROFILE_MAIN10_GUID;
        *format = NV_ENC_BUFFER_FORMAT_YUV420_10BIT;
        *chroma = 1;
        *bit_depth = NV_ENC_BIT_DEPTH_10;
        return 1;
    case 3:
        *codec = NV_ENC_CODEC_HEVC_GUID;
        *profile = NV_ENC_HEVC_PROFILE_FREXT_GUID;
        *format = NV_ENC_BUFFER_FORMAT_YUV444;
        *chroma = 3;
        *bit_depth = NV_ENC_BIT_DEPTH_8;
        return 1;
    case 4:
        *codec = NV_ENC_CODEC_HEVC_GUID;
        *profile = NV_ENC_HEVC_PROFILE_FREXT_GUID;
        *format = NV_ENC_BUFFER_FORMAT_YUV444_10BIT;
        *chroma = 3;
        *bit_depth = NV_ENC_BIT_DEPTH_10;
        return 1;
    case 5:
        *codec = NV_ENC_CODEC_AV1_GUID;
        *profile = NV_ENC_AV1_PROFILE_MAIN_GUID;
        *format = NV_ENC_BUFFER_FORMAT_NV12;
        *chroma = 1;
        *bit_depth = NV_ENC_BIT_DEPTH_8;
        return 1;
    case 6:
        *codec = NV_ENC_CODEC_AV1_GUID;
        *profile = NV_ENC_AV1_PROFILE_MAIN_GUID;
        *format = NV_ENC_BUFFER_FORMAT_YUV420_10BIT;
        *chroma = 1;
        *bit_depth = NV_ENC_BIT_DEPTH_10;
        return 1;
    default:
        return 0;
    }
}

static int replay_required_allocation(uint32_t pitch,
                                      uint32_t height,
                                      NV_ENC_BUFFER_FORMAT format,
                                      uint64_t *required_out) {
    uint64_t rows = height;
    if (format == NV_ENC_BUFFER_FORMAT_NV12 ||
        format == NV_ENC_BUFFER_FORMAT_YUV420_10BIT) {
        rows += (uint64_t)(height + 1u) / 2u;
    } else if (format == NV_ENC_BUFFER_FORMAT_YUV444 ||
               format == NV_ENC_BUFFER_FORMAT_YUV444_10BIT) {
        rows *= 3u;
    } else {
        return 0;
    }
    if (rows != 0 && (uint64_t)pitch > UINT64_MAX / rows) {
        return 0;
    }
    *required_out = (uint64_t)pitch * rows;
    return 1;
}

static uint32_t replay_minimum_row_bytes(uint32_t width,
                                         NV_ENC_BUFFER_FORMAT format) {
    if (format == NV_ENC_BUFFER_FORMAT_YUV420_10BIT ||
        format == NV_ENC_BUFFER_FORMAT_YUV444_10BIT) {
        if (width > UINT32_MAX / 2u) {
            return 0;
        }
        return width * 2u;
    }
    return width;
}

static void replay_configure_codec(NV_ENC_CONFIG *config,
                                   GUID codec,
                                   uint32_t chroma,
                                   NV_ENC_BIT_DEPTH bit_depth) {
    if (replay_guid_equal(&codec, &NV_ENC_CODEC_H264_GUID)) {
        config->encodeCodecConfig.h264Config.chromaFormatIDC = chroma;
        config->encodeCodecConfig.h264Config.inputBitDepth = bit_depth;
        config->encodeCodecConfig.h264Config.outputBitDepth = bit_depth;
        config->encodeCodecConfig.h264Config.idrPeriod =
            NVENC_INFINITE_GOPLENGTH;
    } else if (replay_guid_equal(&codec, &NV_ENC_CODEC_HEVC_GUID)) {
        config->encodeCodecConfig.hevcConfig.chromaFormatIDC = chroma;
        config->encodeCodecConfig.hevcConfig.inputBitDepth = bit_depth;
        config->encodeCodecConfig.hevcConfig.outputBitDepth = bit_depth;
        config->encodeCodecConfig.hevcConfig.idrPeriod =
            NVENC_INFINITE_GOPLENGTH;
    } else {
        config->encodeCodecConfig.av1Config.chromaFormatIDC = chroma;
        config->encodeCodecConfig.av1Config.inputBitDepth = bit_depth;
        config->encodeCodecConfig.av1Config.outputBitDepth = bit_depth;
        config->encodeCodecConfig.av1Config.idrPeriod =
            NVENC_INFINITE_GOPLENGTH;
    }
}

uint32_t replay_nvenc_header_api_major(void) {
    return NVENCAPI_MAJOR_VERSION;
}

uint32_t replay_nvenc_header_api_minor(void) {
    return NVENCAPI_MINOR_VERSION;
}

uint32_t replay_nvenc_header_api_version(void) {
    return NVENCAPI_VERSION;
}

uint32_t replay_nvenc_required_runtime_api_raw(void) {
    return REPLAY_NVENC_REQUIRED_RUNTIME_API_RAW;
}

uint32_t replay_nvenc_runtime_api_major(uint32_t raw) {
    return raw >> 4;
}

uint32_t replay_nvenc_runtime_api_minor(uint32_t raw) {
    return raw & 0x0fu;
}

uint32_t replay_nvenc_min_linux_driver_major(void) {
    return REPLAY_NVENC_MIN_LINUX_DRIVER_MAJOR;
}

uint32_t replay_nvenc_min_cuda_driver_api(void) {
    return REPLAY_NVENC_MIN_CUDA_DRIVER_API;
}

uint32_t replay_nvenc_status_success_class(void) {
    return REPLAY_NVENC_STATUS_SUCCESS;
}

uint32_t replay_nvenc_native_status_success(void) {
    return (uint32_t)NV_ENC_SUCCESS;
}

uint32_t replay_nvenc_status_unsupported_class(void) {
    return REPLAY_NVENC_STATUS_UNSUPPORTED;
}

uint32_t replay_nvenc_status_operational_reject_class(void) {
    return REPLAY_NVENC_STATUS_OPERATIONAL_REJECT;
}

uint32_t replay_nvenc_status_invalid_input_class(void) {
    return REPLAY_NVENC_STATUS_INVALID_INPUT;
}

uint32_t replay_nvenc_acquired_complete_mask(void) {
    return REPLAY_NVENC_ACQUIRED_API | REPLAY_NVENC_ACQUIRED_SESSION |
           REPLAY_NVENC_ACQUIRED_INITIALIZED |
           REPLAY_NVENC_ACQUIRED_REGISTERED | REPLAY_NVENC_ACQUIRED_MAPPED |
           REPLAY_NVENC_ACQUIRED_BITSTREAM |
           REPLAY_NVENC_ACQUIRED_SUBMITTED | REPLAY_NVENC_ACQUIRED_LOCKED |
           REPLAY_NVENC_ACQUIRED_COPIED;
}

uint32_t replay_nvenc_released_complete_mask(void) {
    return REPLAY_NVENC_RELEASED_UNLOCKED |
           REPLAY_NVENC_RELEASED_BITSTREAM | REPLAY_NVENC_RELEASED_UNMAPPED |
           REPLAY_NVENC_RELEASED_UNREGISTERED |
           REPLAY_NVENC_RELEASED_SESSION;
}

uint32_t replay_nvenc_config_complete_mask(void) {
    return REPLAY_NVENC_CONFIG_DIMENSIONS | REPLAY_NVENC_CONFIG_FRAMERATE |
           REPLAY_NVENC_CONFIG_PRESET_P2 | REPLAY_NVENC_CONFIG_TUNING_ULL |
           REPLAY_NVENC_CONFIG_SYNCHRONOUS | REPLAY_NVENC_CONFIG_PTD |
           REPLAY_NVENC_CONFIG_FORCE_IDR_HEADERS |
           REPLAY_NVENC_CONFIG_NO_B_FRAMES |
           REPLAY_NVENC_CONFIG_NO_LOOKAHEAD |
           REPLAY_NVENC_CONFIG_ZERO_REORDER |
           REPLAY_NVENC_CONFIG_ONE_FRAME_VBV;
}

uint32_t replay_nvenc_resource_complete_mask(void) {
    return REPLAY_NVENC_RESOURCE_CUDA_CONTEXT |
           REPLAY_NVENC_RESOURCE_DEVICE_POINTER |
           REPLAY_NVENC_RESOURCE_ALLOCATION_BOUNDS |
           REPLAY_NVENC_RESOURCE_REGISTERED | REPLAY_NVENC_RESOURCE_MAPPED |
           REPLAY_NVENC_RESOURCE_FORMAT_MATCH |
           REPLAY_NVENC_RESOURCE_SUBMITTED;
}

size_t replay_nvenc_sizeof_probe_result(void) {
    return sizeof(struct replay_nvenc_probe_result);
}

size_t replay_nvenc_alignof_probe_result(void) {
    return _Alignof(struct replay_nvenc_probe_result);
}

size_t replay_nvenc_sizeof_function_list(void) {
    return sizeof(NV_ENCODE_API_FUNCTION_LIST);
}

size_t replay_nvenc_alignof_function_list(void) {
    return _Alignof(NV_ENCODE_API_FUNCTION_LIST);
}

size_t replay_nvenc_sizeof_open_session(void) {
    return sizeof(NV_ENC_OPEN_ENCODE_SESSION_EX_PARAMS);
}

size_t replay_nvenc_alignof_open_session(void) {
    return _Alignof(NV_ENC_OPEN_ENCODE_SESSION_EX_PARAMS);
}

size_t replay_nvenc_sizeof_initialize(void) {
    return sizeof(NV_ENC_INITIALIZE_PARAMS);
}

size_t replay_nvenc_alignof_initialize(void) {
    return _Alignof(NV_ENC_INITIALIZE_PARAMS);
}

size_t replay_nvenc_sizeof_config(void) {
    return sizeof(NV_ENC_CONFIG);
}

size_t replay_nvenc_alignof_config(void) {
    return _Alignof(NV_ENC_CONFIG);
}

size_t replay_nvenc_sizeof_rate_control(void) {
    return sizeof(NV_ENC_RC_PARAMS);
}

size_t replay_nvenc_alignof_rate_control(void) {
    return _Alignof(NV_ENC_RC_PARAMS);
}

size_t replay_nvenc_sizeof_preset_config(void) {
    return sizeof(NV_ENC_PRESET_CONFIG);
}

size_t replay_nvenc_alignof_preset_config(void) {
    return _Alignof(NV_ENC_PRESET_CONFIG);
}

size_t replay_nvenc_sizeof_caps_param(void) {
    return sizeof(NV_ENC_CAPS_PARAM);
}

size_t replay_nvenc_alignof_caps_param(void) {
    return _Alignof(NV_ENC_CAPS_PARAM);
}

size_t replay_nvenc_sizeof_register_resource(void) {
    return sizeof(NV_ENC_REGISTER_RESOURCE);
}

size_t replay_nvenc_alignof_register_resource(void) {
    return _Alignof(NV_ENC_REGISTER_RESOURCE);
}

size_t replay_nvenc_sizeof_map_resource(void) {
    return sizeof(NV_ENC_MAP_INPUT_RESOURCE);
}

size_t replay_nvenc_alignof_map_resource(void) {
    return _Alignof(NV_ENC_MAP_INPUT_RESOURCE);
}

size_t replay_nvenc_sizeof_create_bitstream(void) {
    return sizeof(NV_ENC_CREATE_BITSTREAM_BUFFER);
}

size_t replay_nvenc_alignof_create_bitstream(void) {
    return _Alignof(NV_ENC_CREATE_BITSTREAM_BUFFER);
}

size_t replay_nvenc_sizeof_picture(void) {
    return sizeof(NV_ENC_PIC_PARAMS);
}

size_t replay_nvenc_alignof_picture(void) {
    return _Alignof(NV_ENC_PIC_PARAMS);
}

size_t replay_nvenc_sizeof_lock_bitstream(void) {
    return sizeof(NV_ENC_LOCK_BITSTREAM);
}

size_t replay_nvenc_alignof_lock_bitstream(void) {
    return _Alignof(NV_ENC_LOCK_BITSTREAM);
}

uint32_t replay_nvenc_function_list_version(void) {
    return NV_ENCODE_API_FUNCTION_LIST_VER;
}

uint32_t replay_nvenc_open_session_version(void) {
    return NV_ENC_OPEN_ENCODE_SESSION_EX_PARAMS_VER;
}

uint32_t replay_nvenc_initialize_version(void) {
    return NV_ENC_INITIALIZE_PARAMS_VER;
}

uint32_t replay_nvenc_config_version(void) {
    return NV_ENC_CONFIG_VER;
}

uint32_t replay_nvenc_rate_control_version(void) {
    return NV_ENC_RC_PARAMS_VER;
}

uint32_t replay_nvenc_preset_config_version(void) {
    return NV_ENC_PRESET_CONFIG_VER;
}

uint32_t replay_nvenc_caps_param_version(void) {
    return NV_ENC_CAPS_PARAM_VER;
}

uint32_t replay_nvenc_register_resource_version(void) {
    return NV_ENC_REGISTER_RESOURCE_VER;
}

uint32_t replay_nvenc_map_resource_version(void) {
    return NV_ENC_MAP_INPUT_RESOURCE_VER;
}

uint32_t replay_nvenc_create_bitstream_version(void) {
    return NV_ENC_CREATE_BITSTREAM_BUFFER_VER;
}

uint32_t replay_nvenc_picture_version(void) {
    return NV_ENC_PIC_PARAMS_VER;
}

uint32_t replay_nvenc_lock_bitstream_version(void) {
    return NV_ENC_LOCK_BITSTREAM_VER;
}

uint32_t replay_nvenc_signature_mask(void) {
    return 0x00ffffffu;
}

int replay_nvenc_policy_facts(uint32_t position,
                              uint8_t *codec_guid_out,
                              uint8_t *profile_guid_out,
                              uint32_t *format_out,
                              uint32_t *chroma_out,
                              uint32_t *bit_depth_out) {
    GUID codec;
    GUID profile;
    NV_ENC_BUFFER_FORMAT format;
    uint32_t chroma = 0;
    NV_ENC_BIT_DEPTH bit_depth = NV_ENC_BIT_DEPTH_INVALID;
    if (codec_guid_out == NULL || profile_guid_out == NULL ||
        format_out == NULL || chroma_out == NULL || bit_depth_out == NULL ||
        !replay_policy(position, &codec, &profile, &format, &chroma,
                       &bit_depth)) {
        return 0;
    }
    memcpy(codec_guid_out, &codec, sizeof(codec));
    memcpy(profile_guid_out, &profile, sizeof(profile));
    *format_out = (uint32_t)format;
    *chroma_out = chroma;
    *bit_depth_out = (uint32_t)bit_depth;
    return 1;
}

int replay_nvenc_probe_one_frame(
    void *create_instance_fn,
    void *get_max_version_fn,
    void *cuda_context,
    uint64_t cuda_device_ptr,
    uint64_t allocation_size,
    uint32_t width,
    uint32_t height,
    uint32_t pitch,
    uint32_t policy_position,
    uint8_t *bitstream_out,
    uint32_t capacity,
    struct replay_nvenc_probe_result *result) {
    NV_ENCODE_API_FUNCTION_LIST functions;
    NV_ENC_OPEN_ENCODE_SESSION_EX_PARAMS open_params;
    NV_ENC_PRESET_CONFIG preset_config;
    NV_ENC_CONFIG config;
    NV_ENC_INITIALIZE_PARAMS initialize;
    NV_ENC_REGISTER_RESOURCE register_resource;
    NV_ENC_MAP_INPUT_RESOURCE map_resource;
    NV_ENC_CREATE_BITSTREAM_BUFFER create_bitstream;
    NV_ENC_PIC_PARAMS picture;
    NV_ENC_LOCK_BITSTREAM lock;
    replay_nvenc_create_instance_fn create_instance;
    replay_nvenc_get_max_version_fn get_max_version;
    GUID codec;
    GUID profile;
    NV_ENC_BUFFER_FORMAT format;
    NV_ENC_BIT_DEPTH bit_depth;
    uint32_t chroma = 0;
    uint32_t row_bytes = 0;
    uint64_t required_allocation = 0;
    uint32_t runtime_max = 0;
    uint32_t average_bitrate = 80000000u;
    uint32_t one_frame_vbv = (80000000u + 59u) / 60u;
    NVENCSTATUS status = NV_ENC_SUCCESS;
    NVENCSTATUS support_status = NV_ENC_SUCCESS;
    NVENCSTATUS cleanup_status = NV_ENC_SUCCESS;
    void *encoder = NULL;
    NV_ENC_REGISTERED_PTR registered = NULL;
    NV_ENC_INPUT_PTR mapped = NULL;
    NV_ENC_OUTPUT_PTR bitstream = NULL;
    int locked = 0;
    int support = 0;
    int cap_value = 0;

    if (result == NULL) {
        return -1;
    }
    memset(result, 0, sizeof(*result));
    result->struct_size = sizeof(*result);
    result->status_class = REPLAY_NVENC_STATUS_INVALID_INPUT;
    result->nvenc_status = (uint32_t)NV_ENC_ERR_INVALID_PARAM;
    result->cleanup_status = (uint32_t)NV_ENC_SUCCESS;

    if (create_instance_fn == NULL || get_max_version_fn == NULL ||
        cuda_context == NULL || cuda_device_ptr == 0 || allocation_size == 0 ||
        bitstream_out == NULL || capacity == 0 || width != 3840u ||
        height != 2160u || pitch == 0 || (pitch & 3u) != 0 ||
        !replay_policy(policy_position, &codec, &profile, &format, &chroma,
                       &bit_depth)) {
        return 0;
    }
    row_bytes = replay_minimum_row_bytes(width, format);
    if (row_bytes == 0 || pitch < row_bytes ||
        !replay_required_allocation(pitch, height, format,
                                    &required_allocation) ||
        allocation_size < required_allocation) {
        return 0;
    }
    result->resource_mask =
        REPLAY_NVENC_RESOURCE_CUDA_CONTEXT |
        REPLAY_NVENC_RESOURCE_DEVICE_POINTER |
        REPLAY_NVENC_RESOURCE_ALLOCATION_BOUNDS;

    create_instance = (replay_nvenc_create_instance_fn)create_instance_fn;
    get_max_version = (replay_nvenc_get_max_version_fn)get_max_version_fn;
    status = get_max_version(&runtime_max);
    result->runtime_max_api = runtime_max;
    if (status != NV_ENC_SUCCESS) {
        replay_set_failure(result, status,
                           REPLAY_NVENC_STATUS_OPERATIONAL_REJECT);
        return 0;
    }
    if (runtime_max < REPLAY_NVENC_REQUIRED_RUNTIME_API_RAW) {
        replay_set_failure(result, NV_ENC_ERR_INVALID_VERSION,
                           REPLAY_NVENC_STATUS_UNSUPPORTED);
        return 0;
    }

    memset(&functions, 0, sizeof(functions));
    functions.version = NV_ENCODE_API_FUNCTION_LIST_VER;
    status = create_instance(&functions);
    if (status != NV_ENC_SUCCESS) {
        replay_set_failure(result, status, replay_status_class_for(status));
        return 0;
    }
    result->acquired_mask |= REPLAY_NVENC_ACQUIRED_API;
    if (!replay_function_list_complete(&functions)) {
        replay_set_failure(result, NV_ENC_ERR_INVALID_VERSION,
                           REPLAY_NVENC_STATUS_OPERATIONAL_REJECT);
        return 0;
    }

    memset(&open_params, 0, sizeof(open_params));
    open_params.version = NV_ENC_OPEN_ENCODE_SESSION_EX_PARAMS_VER;
    open_params.deviceType = NV_ENC_DEVICE_TYPE_CUDA;
    open_params.device = cuda_context;
    open_params.apiVersion = NVENCAPI_VERSION;
    status = functions.nvEncOpenEncodeSessionEx(&open_params, &encoder);
    if (status != NV_ENC_SUCCESS || encoder == NULL) {
        if (status == NV_ENC_SUCCESS) {
            status = NV_ENC_ERR_INVALID_PTR;
        }
        replay_set_failure(result, status, replay_status_class_for(status));
        return 0;
    }
    result->acquired_mask |= REPLAY_NVENC_ACQUIRED_SESSION;

    support = replay_guid_supported(functions.nvEncGetEncodeGUIDCount,
                                    functions.nvEncGetEncodeGUIDs, encoder,
                                    &codec, &support_status);
    if (support <= 0) {
        replay_set_failure(
            result, support_status,
            support == 0 ? REPLAY_NVENC_STATUS_UNSUPPORTED
                         : replay_status_class_for(support_status));
        goto cleanup;
    }
    support =
        replay_profile_supported(&functions, encoder, codec, &profile,
                                 &support_status);
    if (support <= 0) {
        replay_set_failure(
            result, support_status,
            support == 0 ? REPLAY_NVENC_STATUS_UNSUPPORTED
                         : replay_status_class_for(support_status));
        goto cleanup;
    }
    support = replay_format_supported(&functions, encoder, codec, format,
                                      &support_status);
    if (support <= 0) {
        replay_set_failure(
            result, support_status,
            support == 0 ? REPLAY_NVENC_STATUS_UNSUPPORTED
                         : replay_status_class_for(support_status));
        goto cleanup;
    }
    support = replay_preset_supported(&functions, encoder, codec,
                                      &NV_ENC_PRESET_P2_GUID, &support_status);
    if (support <= 0) {
        replay_set_failure(
            result, support_status,
            support == 0 ? REPLAY_NVENC_STATUS_UNSUPPORTED
                         : replay_status_class_for(support_status));
        goto cleanup;
    }
    if (!replay_capability(&functions, encoder, codec, NV_ENC_CAPS_WIDTH_MAX,
                           &cap_value, &support_status)) {
        replay_set_failure(result, support_status,
                           replay_status_class_for(support_status));
        goto cleanup;
    }
    if (cap_value < (int)width) {
        replay_set_failure(result, NV_ENC_ERR_UNSUPPORTED_PARAM,
                           REPLAY_NVENC_STATUS_UNSUPPORTED);
        goto cleanup;
    }
    if (!replay_capability(&functions, encoder, codec, NV_ENC_CAPS_HEIGHT_MAX,
                           &cap_value, &support_status)) {
        replay_set_failure(result, support_status,
                           replay_status_class_for(support_status));
        goto cleanup;
    }
    if (cap_value < (int)height) {
        replay_set_failure(result, NV_ENC_ERR_UNSUPPORTED_PARAM,
                           REPLAY_NVENC_STATUS_UNSUPPORTED);
        goto cleanup;
    }
    if (chroma == 3u) {
        if (!replay_capability(&functions, encoder, codec,
                               NV_ENC_CAPS_SUPPORT_YUV444_ENCODE, &cap_value,
                               &support_status)) {
            replay_set_failure(result, support_status,
                               replay_status_class_for(support_status));
            goto cleanup;
        }
        if (cap_value == 0) {
            replay_set_failure(result, NV_ENC_ERR_UNSUPPORTED_PARAM,
                               REPLAY_NVENC_STATUS_UNSUPPORTED);
            goto cleanup;
        }
    }
    if (bit_depth == NV_ENC_BIT_DEPTH_10) {
        if (!replay_capability(&functions, encoder, codec,
                               NV_ENC_CAPS_SUPPORT_10BIT_ENCODE, &cap_value,
                               &support_status)) {
            replay_set_failure(result, support_status,
                               replay_status_class_for(support_status));
            goto cleanup;
        }
        if (cap_value == 0) {
            replay_set_failure(result, NV_ENC_ERR_UNSUPPORTED_PARAM,
                               REPLAY_NVENC_STATUS_UNSUPPORTED);
            goto cleanup;
        }
    }

    memset(&preset_config, 0, sizeof(preset_config));
    preset_config.version = NV_ENC_PRESET_CONFIG_VER;
    preset_config.presetCfg.version = NV_ENC_CONFIG_VER;
    status = functions.nvEncGetEncodePresetConfigEx(
        encoder, codec, NV_ENC_PRESET_P2_GUID,
        NV_ENC_TUNING_INFO_ULTRA_LOW_LATENCY, &preset_config);
    if (status != NV_ENC_SUCCESS) {
        replay_set_failure(result, status, replay_status_class_for(status));
        goto cleanup;
    }
    config = preset_config.presetCfg;
    config.version = NV_ENC_CONFIG_VER;
    config.profileGUID = profile;
    config.gopLength = NVENC_INFINITE_GOPLENGTH;
    config.frameIntervalP = 1;
    config.frameFieldMode = NV_ENC_PARAMS_FRAME_FIELD_MODE_FRAME;
    config.mvPrecision = NV_ENC_MV_PRECISION_DEFAULT;
    config.rcParams.version = NV_ENC_RC_PARAMS_VER;
    config.rcParams.rateControlMode = NV_ENC_PARAMS_RC_CBR;
    config.rcParams.averageBitRate = average_bitrate;
    config.rcParams.maxBitRate = average_bitrate;
    config.rcParams.vbvBufferSize = one_frame_vbv;
    config.rcParams.vbvInitialDelay = one_frame_vbv;
    config.rcParams.enableLookahead = 0;
    config.rcParams.lookaheadDepth = 0;
    config.rcParams.lookaheadLevel = NV_ENC_LOOKAHEAD_LEVEL_0;
    config.rcParams.zeroReorderDelay = 1;
    config.rcParams.multiPass = NV_ENC_MULTI_PASS_DISABLED;
    replay_configure_codec(&config, codec, chroma, bit_depth);

    memset(&initialize, 0, sizeof(initialize));
    initialize.version = NV_ENC_INITIALIZE_PARAMS_VER;
    initialize.encodeGUID = codec;
    initialize.presetGUID = NV_ENC_PRESET_P2_GUID;
    initialize.encodeWidth = width;
    initialize.encodeHeight = height;
    initialize.darWidth = width;
    initialize.darHeight = height;
    initialize.frameRateNum = 60;
    initialize.frameRateDen = 1;
    initialize.enableEncodeAsync = 0;
    initialize.enablePTD = 1;
    initialize.splitEncodeMode = NV_ENC_SPLIT_DISABLE_MODE;
    initialize.encodeConfig = &config;
    initialize.maxEncodeWidth = width;
    initialize.maxEncodeHeight = height;
    initialize.tuningInfo = NV_ENC_TUNING_INFO_ULTRA_LOW_LATENCY;
    result->config_mask =
        REPLAY_NVENC_CONFIG_DIMENSIONS | REPLAY_NVENC_CONFIG_FRAMERATE |
        REPLAY_NVENC_CONFIG_PRESET_P2 | REPLAY_NVENC_CONFIG_TUNING_ULL |
        REPLAY_NVENC_CONFIG_SYNCHRONOUS | REPLAY_NVENC_CONFIG_PTD |
        REPLAY_NVENC_CONFIG_FORCE_IDR_HEADERS |
        REPLAY_NVENC_CONFIG_NO_B_FRAMES |
        REPLAY_NVENC_CONFIG_NO_LOOKAHEAD |
        REPLAY_NVENC_CONFIG_ZERO_REORDER |
        REPLAY_NVENC_CONFIG_ONE_FRAME_VBV;

    status = functions.nvEncInitializeEncoder(encoder, &initialize);
    if (status != NV_ENC_SUCCESS) {
        replay_set_failure(result, status, replay_status_class_for(status));
        goto cleanup;
    }
    result->acquired_mask |= REPLAY_NVENC_ACQUIRED_INITIALIZED;

    memset(&register_resource, 0, sizeof(register_resource));
    register_resource.version = NV_ENC_REGISTER_RESOURCE_VER;
    register_resource.resourceType =
        NV_ENC_INPUT_RESOURCE_TYPE_CUDADEVICEPTR;
    register_resource.width = width;
    register_resource.height = height;
    register_resource.pitch = pitch;
    register_resource.resourceToRegister =
        (void *)(uintptr_t)cuda_device_ptr;
    register_resource.bufferFormat = format;
    register_resource.bufferUsage = NV_ENC_INPUT_IMAGE;
    status =
        functions.nvEncRegisterResource(encoder, &register_resource);
    if (status != NV_ENC_SUCCESS ||
        register_resource.registeredResource == NULL) {
        if (status == NV_ENC_SUCCESS) {
            status = NV_ENC_ERR_INVALID_PTR;
        }
        replay_set_failure(result, status, replay_status_class_for(status));
        goto cleanup;
    }
    registered = register_resource.registeredResource;
    result->acquired_mask |= REPLAY_NVENC_ACQUIRED_REGISTERED;
    result->resource_mask |= REPLAY_NVENC_RESOURCE_REGISTERED;

    memset(&map_resource, 0, sizeof(map_resource));
    map_resource.version = NV_ENC_MAP_INPUT_RESOURCE_VER;
    map_resource.registeredResource = registered;
    status = functions.nvEncMapInputResource(encoder, &map_resource);
    if (status != NV_ENC_SUCCESS || map_resource.mappedResource == NULL) {
        if (status == NV_ENC_SUCCESS) {
            status = NV_ENC_ERR_INVALID_PTR;
        }
        replay_set_failure(result, status, replay_status_class_for(status));
        goto cleanup;
    }
    mapped = map_resource.mappedResource;
    result->mapped_format = (uint32_t)map_resource.mappedBufferFmt;
    result->acquired_mask |= REPLAY_NVENC_ACQUIRED_MAPPED;
    result->resource_mask |= REPLAY_NVENC_RESOURCE_MAPPED;
    if (map_resource.mappedBufferFmt != format) {
        replay_set_failure(result, NV_ENC_ERR_MAP_FAILED,
                           REPLAY_NVENC_STATUS_OPERATIONAL_REJECT);
        goto cleanup;
    }
    result->resource_mask |= REPLAY_NVENC_RESOURCE_FORMAT_MATCH;

    memset(&create_bitstream, 0, sizeof(create_bitstream));
    create_bitstream.version = NV_ENC_CREATE_BITSTREAM_BUFFER_VER;
    status =
        functions.nvEncCreateBitstreamBuffer(encoder, &create_bitstream);
    if (status != NV_ENC_SUCCESS ||
        create_bitstream.bitstreamBuffer == NULL) {
        if (status == NV_ENC_SUCCESS) {
            status = NV_ENC_ERR_INVALID_PTR;
        }
        replay_set_failure(result, status, replay_status_class_for(status));
        goto cleanup;
    }
    bitstream = create_bitstream.bitstreamBuffer;
    result->acquired_mask |= REPLAY_NVENC_ACQUIRED_BITSTREAM;

    memset(&picture, 0, sizeof(picture));
    picture.version = NV_ENC_PIC_PARAMS_VER;
    picture.inputWidth = width;
    picture.inputHeight = height;
    picture.inputPitch = pitch;
    picture.encodePicFlags =
        NV_ENC_PIC_FLAG_FORCEIDR | NV_ENC_PIC_FLAG_OUTPUT_SPSPPS;
    picture.frameIdx = 0;
    picture.inputTimeStamp = 0;
    picture.inputDuration = 1;
    picture.inputBuffer = mapped;
    picture.outputBitstream = bitstream;
    picture.bufferFmt = map_resource.mappedBufferFmt;
    picture.pictureStruct = NV_ENC_PIC_STRUCT_FRAME;
    status = functions.nvEncEncodePicture(encoder, &picture);
    if (status != NV_ENC_SUCCESS) {
        replay_set_failure(result, status, replay_status_class_for(status));
        goto cleanup;
    }
    result->acquired_mask |= REPLAY_NVENC_ACQUIRED_SUBMITTED;
    result->resource_mask |= REPLAY_NVENC_RESOURCE_SUBMITTED;

    memset(&lock, 0, sizeof(lock));
    lock.version = NV_ENC_LOCK_BITSTREAM_VER;
    lock.doNotWait = 0;
    lock.outputBitstream = bitstream;
    status = functions.nvEncLockBitstream(encoder, &lock);
    if (status != NV_ENC_SUCCESS || lock.bitstreamBufferPtr == NULL ||
        lock.bitstreamSizeInBytes == 0) {
        if (status == NV_ENC_SUCCESS) {
            status = NV_ENC_ERR_GENERIC;
        }
        replay_set_failure(result, status, replay_status_class_for(status));
        goto cleanup;
    }
    locked = 1;
    result->acquired_mask |= REPLAY_NVENC_ACQUIRED_LOCKED;
    result->bitstream_size = lock.bitstreamSizeInBytes;
    result->picture_type = (uint32_t)lock.pictureType;
    if (lock.bitstreamSizeInBytes > capacity) {
        replay_set_failure(result, NV_ENC_ERR_NOT_ENOUGH_BUFFER,
                           REPLAY_NVENC_STATUS_OPERATIONAL_REJECT);
        goto cleanup;
    }
    memcpy(bitstream_out, lock.bitstreamBufferPtr,
           lock.bitstreamSizeInBytes);
    result->acquired_mask |= REPLAY_NVENC_ACQUIRED_COPIED;
    result->status_class = REPLAY_NVENC_STATUS_SUCCESS;
    result->nvenc_status = (uint32_t)NV_ENC_SUCCESS;

cleanup:
    if (locked) {
        cleanup_status = functions.nvEncUnlockBitstream(encoder, bitstream);
        if (cleanup_status == NV_ENC_SUCCESS) {
            result->released_mask |= REPLAY_NVENC_RELEASED_UNLOCKED;
        } else if (result->cleanup_status == (uint32_t)NV_ENC_SUCCESS) {
            result->cleanup_status = (uint32_t)cleanup_status;
        }
    }
    if (bitstream != NULL) {
        cleanup_status =
            functions.nvEncDestroyBitstreamBuffer(encoder, bitstream);
        if (cleanup_status == NV_ENC_SUCCESS) {
            result->released_mask |= REPLAY_NVENC_RELEASED_BITSTREAM;
        } else if (result->cleanup_status == (uint32_t)NV_ENC_SUCCESS) {
            result->cleanup_status = (uint32_t)cleanup_status;
        }
    }
    if (mapped != NULL) {
        cleanup_status =
            functions.nvEncUnmapInputResource(encoder, mapped);
        if (cleanup_status == NV_ENC_SUCCESS) {
            result->released_mask |= REPLAY_NVENC_RELEASED_UNMAPPED;
        } else if (result->cleanup_status == (uint32_t)NV_ENC_SUCCESS) {
            result->cleanup_status = (uint32_t)cleanup_status;
        }
    }
    if (registered != NULL) {
        cleanup_status =
            functions.nvEncUnregisterResource(encoder, registered);
        if (cleanup_status == NV_ENC_SUCCESS) {
            result->released_mask |= REPLAY_NVENC_RELEASED_UNREGISTERED;
        } else if (result->cleanup_status == (uint32_t)NV_ENC_SUCCESS) {
            result->cleanup_status = (uint32_t)cleanup_status;
        }
    }
    if (encoder != NULL) {
        cleanup_status = functions.nvEncDestroyEncoder(encoder);
        if (cleanup_status == NV_ENC_SUCCESS) {
            result->released_mask |= REPLAY_NVENC_RELEASED_SESSION;
        } else if (result->cleanup_status == (uint32_t)NV_ENC_SUCCESS) {
            result->cleanup_status = (uint32_t)cleanup_status;
        }
    }
    if (result->cleanup_status != (uint32_t)NV_ENC_SUCCESS &&
        result->status_class == REPLAY_NVENC_STATUS_SUCCESS) {
        result->status_class = REPLAY_NVENC_STATUS_OPERATIONAL_REJECT;
        result->nvenc_status = result->cleanup_status;
    }
    return 0;
}
