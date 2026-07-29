/*
 * Project-owned ABI oracle for operator-supplied NvFBC 1.9 and CUDA Driver
 * API headers. It contains no vendored NVIDIA declarations, source paths,
 * native calls, or capture implementation.
 */
#include <NvFBC.h>
#include <cuda.h>
#include <cudaTypedefs.h>
#include <stddef.h>

#define REPLAY_LAYOUT(prefix, type)                                          \
    size_t replay_nvfbc_sizeof_##prefix(void) { return sizeof(type); }       \
    size_t replay_nvfbc_alignof_##prefix(void) { return _Alignof(type); }

#define REPLAY_OFFSET(prefix, type, field)                                   \
    size_t replay_nvfbc_offsetof_##prefix##_##field(void) {                  \
        return offsetof(type, field);                                        \
    }

#define REPLAY_NVFBC_U32(name, value)                                        \
    unsigned int replay_nvfbc_##name(void) { return (unsigned int)(value); }

#define REPLAY_CUDA_U32(name, value)                                         \
    unsigned int replay_cuda_##name(void) { return (unsigned int)(value); }

#define REPLAY_ASSERT_NVFBC_FUNCTION_MEMBER(field, pfn_type)                 \
    _Static_assert(                                                          \
        __builtin_types_compatible_p(                                        \
            __typeof__(((NVFBC_API_FUNCTION_LIST *)0)->field), pfn_type),    \
        "NvFBC function member signature changed: " #field)

#define REPLAY_ASSERT_NVFBC_ENTRY_POINT(pfn_type, expression)                \
    _Static_assert(                                                          \
        __builtin_types_compatible_p(pfn_type, __typeof__(expression)),      \
        "NvFBC entry-point signature changed")

#define REPLAY_ASSERT_CUDA_SIGNATURE(pfn_type, expression)                    \
    _Static_assert(                                                          \
        __builtin_types_compatible_p(pfn_type, __typeof__(expression)),      \
        "CUDA historical function signature changed")

_Static_assert(NVFBC_VERSION_MAJOR == 1, "unsupported NvFBC major version");
_Static_assert(NVFBC_VERSION_MINOR == 9, "unsupported NvFBC minor version");
_Static_assert(NVFBC_VERSION == 0x109u, "unsupported NvFBC API version");
_Static_assert(NVFBC_CREATE_HANDLE_PARAMS_VER == 0x09030070u,
               "NvFBC create-handle version changed");
_Static_assert(NVFBC_DESTROY_HANDLE_PARAMS_VER == 0x09010004u,
               "NvFBC destroy-handle version changed");
_Static_assert(NVFBC_GET_STATUS_PARAMS_VER == 0x090403a8u,
               "NvFBC status version changed");
_Static_assert(NVFBC_CREATE_CAPTURE_SESSION_PARAMS_VER == 0x0907004cu,
               "NvFBC create-session version changed");
_Static_assert(NVFBC_DESTROY_CAPTURE_SESSION_PARAMS_VER == 0x09010004u,
               "NvFBC destroy-session version changed");
_Static_assert(NVFBC_BIND_CONTEXT_PARAMS_VER == 0x09010004u,
               "NvFBC bind-context version changed");
_Static_assert(NVFBC_RELEASE_CONTEXT_PARAMS_VER == 0x09010004u,
               "NvFBC release-context version changed");
_Static_assert(NVFBC_TOSYS_SETUP_PARAMS_VER == 0x09030030u,
               "NvFBC ToSys setup version changed");
_Static_assert(NVFBC_TOSYS_GRAB_FRAME_PARAMS_VER == 0x09020018u,
               "NvFBC ToSys grab version changed");
_Static_assert(NVFBC_TOCUDA_SETUP_PARAMS_VER == 0x09010008u,
               "NvFBC ToCUDA setup version changed");
_Static_assert(NVFBC_TOCUDA_GRAB_FRAME_PARAMS_VER == 0x09020020u,
               "NvFBC ToCUDA grab version changed");
_Static_assert(NVFBC_TOGL_SETUP_PARAMS_VER == 0x09020038u,
               "NvFBC ToGL setup version changed");
_Static_assert(NVFBC_TOGL_GRAB_FRAME_PARAMS_VER == 0x09020020u,
               "NvFBC ToGL grab version changed");
_Static_assert(NVFBC_COMPOSITE_CURSOR_PARAMS_VER == 0x09010008u,
               "NvFBC composite-cursor version changed");
_Static_assert(NVFBC_FALSE == 0 && NVFBC_TRUE == 1,
               "NvFBC boolean constants changed");
_Static_assert(
    NVFBC_SUCCESS == 0 && NVFBC_ERR_API_VERSION == 1 &&
        NVFBC_ERR_INTERNAL == 2 && NVFBC_ERR_INVALID_PARAM == 3 &&
        NVFBC_ERR_INVALID_PTR == 4 && NVFBC_ERR_INVALID_HANDLE == 5 &&
        NVFBC_ERR_MAX_CLIENTS == 6 && NVFBC_ERR_UNSUPPORTED == 7 &&
        NVFBC_ERR_OUT_OF_MEMORY == 8 && NVFBC_ERR_BAD_REQUEST == 9 &&
        NVFBC_ERR_X == 10 && NVFBC_ERR_GLX == 11 && NVFBC_ERR_GL == 12 &&
        NVFBC_ERR_CUDA == 13 && NVFBC_ERR_ENCODER == 14 &&
        NVFBC_ERR_CONTEXT == 15 && NVFBC_ERR_MUST_RECREATE == 16 &&
        NVFBC_ERR_VULKAN == 17 && NVFBC_ERR_EGL == 18 &&
        NVFBC_ERR_DBUS == 19 && NVFBC_ERR_PIPEWIRE == 20 &&
        NVFBC_ERR_DRM == 21,
    "NvFBC status constants changed");
_Static_assert(NVFBC_CAPTURE_TO_SYS == 0 &&
                   NVFBC_CAPTURE_SHARED_CUDA == 1 &&
                   NVFBC_CAPTURE_TO_GL == 3,
               "NvFBC capture-type constants changed");
_Static_assert(NVFBC_TRACKING_DEFAULT == 0 &&
                   NVFBC_TRACKING_OUTPUT == 1 &&
                   NVFBC_TRACKING_SCREEN == 2,
               "NvFBC tracking constants changed");
_Static_assert(NVFBC_BUFFER_FORMAT_ARGB == 0 &&
                   NVFBC_BUFFER_FORMAT_RGB == 1 &&
                   NVFBC_BUFFER_FORMAT_NV12 == 2 &&
                   NVFBC_BUFFER_FORMAT_YUV444P == 3 &&
                   NVFBC_BUFFER_FORMAT_RGBA == 4 &&
                   NVFBC_BUFFER_FORMAT_BGRA == 5,
               "NvFBC buffer-format constants changed");
_Static_assert(NVFBC_BACKEND_AUTO == 0 && NVFBC_BACKEND_X11 == 1 &&
                   NVFBC_BACKEND_PIPEWIRE == 2 &&
                   NVFBC_BACKEND_DIRECT == 3,
               "NvFBC backend constants changed");
_Static_assert(NVFBC_TOCUDA_GRAB_FLAGS_NOFLAGS == 0 &&
                   NVFBC_TOCUDA_GRAB_FLAGS_NOWAIT == 1 &&
                   NVFBC_TOCUDA_GRAB_FLAGS_FORCE_REFRESH == 2 &&
                   NVFBC_TOCUDA_GRAB_FLAGS_NOWAIT_IF_NEW_FRAME_READY == 4,
               "NvFBC ToCUDA flag constants changed");
_Static_assert(NVFBC_OUTPUT_MAX == 5 && NVFBC_OUTPUT_NAME_LEN == 128 &&
                   NVFBC_PORTAL_RESTORE_TOKEN_LEN == 64 &&
                   NVFBC_DIRECT_MAX_CAPTURE_TARGETS == 10,
               "NvFBC fixed-array bounds changed");

_Static_assert(sizeof(NVFBC_BOX) == 16u, "NvFBC box layout changed");
_Static_assert(_Alignof(NVFBC_BOX) == 4u, "NvFBC box alignment changed");
_Static_assert(sizeof(NVFBC_SESSION_HANDLE) == 8u,
               "NvFBC session-handle representation changed");
_Static_assert(sizeof(NVFBCSTATUS) == 4u,
               "NvFBC status representation changed");
_Static_assert(sizeof(NVFBC_BOOL) == 4u,
               "NvFBC boolean representation changed");
_Static_assert(sizeof(NVFBC_CAPTURE_TYPE) == 4u,
               "NvFBC capture-type representation changed");
_Static_assert(sizeof(NVFBC_TRACKING_TYPE) == 4u,
               "NvFBC tracking-type representation changed");
_Static_assert(sizeof(NVFBC_BUFFER_FORMAT) == 4u,
               "NvFBC buffer-format representation changed");
_Static_assert(sizeof(NVFBC_BACKEND) == 4u,
               "NvFBC backend representation changed");
_Static_assert(sizeof(NVFBC_TOCUDA_FLAGS) == 4u,
               "NvFBC ToCUDA flags representation changed");
_Static_assert(sizeof(NVFBC_SIZE) == 8u, "NvFBC size layout changed");
_Static_assert(_Alignof(NVFBC_SIZE) == 4u, "NvFBC size alignment changed");
_Static_assert(sizeof(NVFBC_RANDR_OUTPUT_INFO) == 148u,
               "NvFBC RandR output layout changed");
_Static_assert(_Alignof(NVFBC_RANDR_OUTPUT_INFO) == 4u,
               "NvFBC RandR output alignment changed");
_Static_assert(sizeof(NVFBC_FRAME_GRAB_INFO) == 56u,
               "NvFBC frame info layout changed");
_Static_assert(_Alignof(NVFBC_FRAME_GRAB_INFO) == 8u,
               "NvFBC frame info alignment changed");
_Static_assert(sizeof(NVFBC_CREATE_HANDLE_PARAMS) == 112u,
               "NvFBC create-handle layout changed");
_Static_assert(_Alignof(NVFBC_CREATE_HANDLE_PARAMS) == 8u,
               "NvFBC create-handle alignment changed");
_Static_assert(sizeof(NVFBC_GET_STATUS_PARAMS) == 936u,
               "NvFBC status layout changed");
_Static_assert(_Alignof(NVFBC_GET_STATUS_PARAMS) == 4u,
               "NvFBC status alignment changed");
_Static_assert(sizeof(NVFBC_CREATE_CAPTURE_SESSION_PARAMS) == 76u,
               "NvFBC create-session layout changed");
_Static_assert(_Alignof(NVFBC_CREATE_CAPTURE_SESSION_PARAMS) == 4u,
               "NvFBC create-session alignment changed");
_Static_assert(sizeof(NVFBC_TOCUDA_SETUP_PARAMS) == 8u,
               "NvFBC ToCUDA setup layout changed");
_Static_assert(_Alignof(NVFBC_TOCUDA_SETUP_PARAMS) == 4u,
               "NvFBC ToCUDA setup alignment changed");
_Static_assert(sizeof(NVFBC_TOCUDA_GRAB_FRAME_PARAMS) == 32u,
               "NvFBC ToCUDA grab layout changed");
_Static_assert(_Alignof(NVFBC_TOCUDA_GRAB_FRAME_PARAMS) == 8u,
               "NvFBC ToCUDA grab alignment changed");
_Static_assert(sizeof(NVFBC_DESTROY_HANDLE_PARAMS) == 4u,
               "NvFBC destroy-handle layout changed");
_Static_assert(sizeof(NVFBC_DESTROY_CAPTURE_SESSION_PARAMS) == 4u,
               "NvFBC destroy-session layout changed");
_Static_assert(sizeof(NVFBC_BIND_CONTEXT_PARAMS) == 4u,
               "NvFBC bind-context layout changed");
_Static_assert(sizeof(NVFBC_RELEASE_CONTEXT_PARAMS) == 4u,
               "NvFBC release-context layout changed");
_Static_assert(sizeof(NVFBC_API_FUNCTION_LIST) == 184u,
               "NvFBC function-list layout changed");
_Static_assert(_Alignof(NVFBC_API_FUNCTION_LIST) == 8u,
               "NvFBC function-list alignment changed");

_Static_assert(offsetof(NVFBC_API_FUNCTION_LIST, dwVersion) == 0u,
               "NvFBC function-list version offset changed");
_Static_assert(offsetof(NVFBC_API_FUNCTION_LIST, nvFBCGetLastErrorStr) == 8u,
               "NvFBC function-list last-error offset changed");
_Static_assert(offsetof(NVFBC_API_FUNCTION_LIST, nvFBCCreateHandle) == 16u,
               "NvFBC function-list create-handle offset changed");
_Static_assert(offsetof(NVFBC_API_FUNCTION_LIST, nvFBCDestroyHandle) == 24u,
               "NvFBC function-list destroy-handle offset changed");
_Static_assert(offsetof(NVFBC_API_FUNCTION_LIST, nvFBCGetStatus) == 32u,
               "NvFBC function-list status offset changed");
_Static_assert(
    offsetof(NVFBC_API_FUNCTION_LIST, nvFBCCreateCaptureSession) == 40u,
    "NvFBC function-list create-session offset changed");
_Static_assert(
    offsetof(NVFBC_API_FUNCTION_LIST, nvFBCDestroyCaptureSession) == 48u,
    "NvFBC function-list destroy-session offset changed");
_Static_assert(offsetof(NVFBC_API_FUNCTION_LIST, nvFBCToSysSetUp) == 56u,
               "NvFBC function-list ToSys-setup offset changed");
_Static_assert(offsetof(NVFBC_API_FUNCTION_LIST, nvFBCToSysGrabFrame) == 64u,
               "NvFBC function-list ToSys-grab offset changed");
_Static_assert(offsetof(NVFBC_API_FUNCTION_LIST, nvFBCToCudaSetUp) == 72u,
               "NvFBC function-list ToCUDA-setup offset changed");
_Static_assert(offsetof(NVFBC_API_FUNCTION_LIST, nvFBCToCudaGrabFrame) == 80u,
               "NvFBC function-list ToCUDA-grab offset changed");
_Static_assert(offsetof(NVFBC_API_FUNCTION_LIST, pad1) == 88u,
               "NvFBC function-list pad1 offset changed");
_Static_assert(offsetof(NVFBC_API_FUNCTION_LIST, pad2) == 96u,
               "NvFBC function-list pad2 offset changed");
_Static_assert(offsetof(NVFBC_API_FUNCTION_LIST, pad3) == 104u,
               "NvFBC function-list pad3 offset changed");
_Static_assert(offsetof(NVFBC_API_FUNCTION_LIST, nvFBCBindContext) == 112u,
               "NvFBC function-list bind offset changed");
_Static_assert(offsetof(NVFBC_API_FUNCTION_LIST, nvFBCReleaseContext) == 120u,
               "NvFBC function-list release offset changed");
_Static_assert(offsetof(NVFBC_API_FUNCTION_LIST, pad4) == 128u,
               "NvFBC function-list pad4 offset changed");
_Static_assert(offsetof(NVFBC_API_FUNCTION_LIST, pad5) == 136u,
               "NvFBC function-list pad5 offset changed");
_Static_assert(offsetof(NVFBC_API_FUNCTION_LIST, pad6) == 144u,
               "NvFBC function-list pad6 offset changed");
_Static_assert(offsetof(NVFBC_API_FUNCTION_LIST, pad7) == 152u,
               "NvFBC function-list pad7 offset changed");
_Static_assert(offsetof(NVFBC_API_FUNCTION_LIST, nvFBCToGLSetUp) == 160u,
               "NvFBC function-list ToGL-setup offset changed");
_Static_assert(offsetof(NVFBC_API_FUNCTION_LIST, nvFBCToGLGrabFrame) == 168u,
               "NvFBC function-list ToGL-grab offset changed");
_Static_assert(offsetof(NVFBC_API_FUNCTION_LIST, nvFBCCompositeCursor) == 176u,
               "NvFBC function-list composite-cursor offset changed");

REPLAY_ASSERT_NVFBC_ENTRY_POINT(PNVFBCCREATEINSTANCE, &NvFBCCreateInstance);
REPLAY_ASSERT_NVFBC_FUNCTION_MEMBER(nvFBCGetLastErrorStr,
                                    PNVFBCGETLASTERRORSTR);
REPLAY_ASSERT_NVFBC_FUNCTION_MEMBER(nvFBCCreateHandle, PNVFBCCREATEHANDLE);
REPLAY_ASSERT_NVFBC_FUNCTION_MEMBER(nvFBCDestroyHandle, PNVFBCDESTROYHANDLE);
REPLAY_ASSERT_NVFBC_FUNCTION_MEMBER(nvFBCGetStatus, PNVFBCGETSTATUS);
REPLAY_ASSERT_NVFBC_FUNCTION_MEMBER(nvFBCCreateCaptureSession,
                                    PNVFBCCREATECAPTURESESSION);
REPLAY_ASSERT_NVFBC_FUNCTION_MEMBER(nvFBCDestroyCaptureSession,
                                    PNVFBCDESTROYCAPTURESESSION);
REPLAY_ASSERT_NVFBC_FUNCTION_MEMBER(nvFBCToSysSetUp, PNVFBCTOSYSSETUP);
REPLAY_ASSERT_NVFBC_FUNCTION_MEMBER(nvFBCToSysGrabFrame,
                                    PNVFBCTOSYSGRABFRAME);
REPLAY_ASSERT_NVFBC_FUNCTION_MEMBER(nvFBCToCudaSetUp, PNVFBCTOCUDASETUP);
REPLAY_ASSERT_NVFBC_FUNCTION_MEMBER(nvFBCToCudaGrabFrame,
                                    PNVFBCTOCUDAGRABFRAME);
REPLAY_ASSERT_NVFBC_FUNCTION_MEMBER(nvFBCBindContext, PNVFBCBINDCONTEXT);
REPLAY_ASSERT_NVFBC_FUNCTION_MEMBER(nvFBCReleaseContext,
                                    PNVFBCRELEASECONTEXT);
REPLAY_ASSERT_NVFBC_FUNCTION_MEMBER(nvFBCToGLSetUp, PNVFBCTOGLSETUP);
REPLAY_ASSERT_NVFBC_FUNCTION_MEMBER(nvFBCToGLGrabFrame,
                                    PNVFBCTOGLGRABFRAME);
REPLAY_ASSERT_NVFBC_FUNCTION_MEMBER(nvFBCCompositeCursor,
                                    PNVFBCCOMPOSITECURSOR);

_Static_assert(sizeof(CUcontext) == 8u, "CUDA context representation changed");
_Static_assert(_Alignof(CUcontext) == 8u, "CUDA context alignment changed");
_Static_assert(sizeof(CUdeviceptr) == 8u,
               "CUDA device pointer representation changed");
_Static_assert(_Alignof(CUdeviceptr) == 8u,
               "CUDA device pointer alignment changed");
_Static_assert(sizeof(CUdevice) == 4u, "CUDA device representation changed");
_Static_assert(_Alignof(CUdevice) == 4u, "CUDA device alignment changed");
_Static_assert(sizeof(CUuuid) == 16u, "CUDA UUID representation changed");
_Static_assert(CUDA_VERSION == 13030, "unsupported CUDA header version");
_Static_assert(CUDA_SUCCESS == 0, "CUDA success constant changed");
_Static_assert(CU_GET_PROC_ADDRESS_LEGACY_STREAM == 1,
               "CUDA legacy-stream lookup flag changed");
_Static_assert(CU_POINTER_ATTRIBUTE_CONTEXT == 1 &&
                   CU_POINTER_ATTRIBUTE_MEMORY_TYPE == 2 &&
                   CU_POINTER_ATTRIBUTE_DEVICE_POINTER == 3 &&
                   CU_POINTER_ATTRIBUTE_HOST_POINTER == 4 &&
                   CU_POINTER_ATTRIBUTE_SYNC_MEMOPS == 6 &&
                   CU_POINTER_ATTRIBUTE_BUFFER_ID == 7 &&
                   CU_POINTER_ATTRIBUTE_IS_MANAGED == 8 &&
                   CU_POINTER_ATTRIBUTE_DEVICE_ORDINAL == 9 &&
                   CU_POINTER_ATTRIBUTE_RANGE_START_ADDR == 11 &&
                   CU_POINTER_ATTRIBUTE_RANGE_SIZE == 12 &&
                   CU_POINTER_ATTRIBUTE_MAPPED == 13,
               "CUDA pointer-attribute constants changed");
_Static_assert(CU_MEMORYTYPE_HOST == 1 && CU_MEMORYTYPE_DEVICE == 2 &&
                   CU_MEMORYTYPE_ARRAY == 3 && CU_MEMORYTYPE_UNIFIED == 4,
               "CUDA memory-type constants changed");

REPLAY_ASSERT_CUDA_SIGNATURE(PFN_cuGetProcAddress_v12000,
                             &cuGetProcAddress);
REPLAY_ASSERT_CUDA_SIGNATURE(PFN_cuInit_v2000, &cuInit);
REPLAY_ASSERT_CUDA_SIGNATURE(PFN_cuDriverGetVersion_v2020,
                             &cuDriverGetVersion);
REPLAY_ASSERT_CUDA_SIGNATURE(PFN_cuDeviceGetByPCIBusId_v4010,
                             &cuDeviceGetByPCIBusId);
REPLAY_ASSERT_CUDA_SIGNATURE(PFN_cuDeviceGetPCIBusId_v4010,
                             &cuDeviceGetPCIBusId);
REPLAY_ASSERT_CUDA_SIGNATURE(PFN_cuDeviceGetUuid_v11040,
                             &cuDeviceGetUuid);
_Static_assert(
    __builtin_types_compatible_p(
        PFN_cuCtxCreate_v3020,
        CUresult(CUDAAPI *)(CUcontext *, unsigned int, CUdevice_v1)),
    "CUDA cuCtxCreate_v2 historical signature changed");
REPLAY_ASSERT_CUDA_SIGNATURE(PFN_cuCtxGetCurrent_v4000,
                             &cuCtxGetCurrent);
REPLAY_ASSERT_CUDA_SIGNATURE(PFN_cuCtxGetDevice_v2000, &cuCtxGetDevice);
REPLAY_ASSERT_CUDA_SIGNATURE(PFN_cuPointerGetAttribute_v4000,
                             &cuPointerGetAttribute);
REPLAY_ASSERT_CUDA_SIGNATURE(PFN_cuCtxDestroy_v4000, &cuCtxDestroy);
REPLAY_ASSERT_CUDA_SIGNATURE(PFN_cuMemAlloc_v3020, &cuMemAlloc);
REPLAY_ASSERT_CUDA_SIGNATURE(PFN_cuMemcpyDtoD_v3020, &cuMemcpyDtoD);
REPLAY_ASSERT_CUDA_SIGNATURE(PFN_cuCtxSynchronize_v2000,
                             &cuCtxSynchronize);
REPLAY_ASSERT_CUDA_SIGNATURE(PFN_cuMemFree_v3020, &cuMemFree);

REPLAY_LAYOUT(box, NVFBC_BOX)
REPLAY_OFFSET(box, NVFBC_BOX, x)
REPLAY_OFFSET(box, NVFBC_BOX, y)
REPLAY_OFFSET(box, NVFBC_BOX, w)
REPLAY_OFFSET(box, NVFBC_BOX, h)

REPLAY_LAYOUT(session_handle, NVFBC_SESSION_HANDLE)
REPLAY_LAYOUT(status, NVFBCSTATUS)
REPLAY_LAYOUT(bool, NVFBC_BOOL)
REPLAY_LAYOUT(capture_type, NVFBC_CAPTURE_TYPE)
REPLAY_LAYOUT(tracking_type, NVFBC_TRACKING_TYPE)
REPLAY_LAYOUT(buffer_format, NVFBC_BUFFER_FORMAT)
REPLAY_LAYOUT(backend, NVFBC_BACKEND)
REPLAY_LAYOUT(tocuda_flags, NVFBC_TOCUDA_FLAGS)

REPLAY_LAYOUT(size, NVFBC_SIZE)
REPLAY_OFFSET(size, NVFBC_SIZE, w)
REPLAY_OFFSET(size, NVFBC_SIZE, h)

REPLAY_LAYOUT(randr_output_info, NVFBC_RANDR_OUTPUT_INFO)
REPLAY_OFFSET(randr_output_info, NVFBC_RANDR_OUTPUT_INFO, dwId)
REPLAY_OFFSET(randr_output_info, NVFBC_RANDR_OUTPUT_INFO, name)
REPLAY_OFFSET(randr_output_info, NVFBC_RANDR_OUTPUT_INFO, trackedBox)

size_t replay_nvfbc_sizeof_frame_grab_info(void) {
    return sizeof(NVFBC_FRAME_GRAB_INFO);
}
size_t replay_nvfbc_alignof_frame_grab_info(void) {
    return _Alignof(NVFBC_FRAME_GRAB_INFO);
}
REPLAY_OFFSET(frame_grab_info, NVFBC_FRAME_GRAB_INFO, dwWidth)
REPLAY_OFFSET(frame_grab_info, NVFBC_FRAME_GRAB_INFO, dwHeight)
REPLAY_OFFSET(frame_grab_info, NVFBC_FRAME_GRAB_INFO, dwByteSize)
REPLAY_OFFSET(frame_grab_info, NVFBC_FRAME_GRAB_INFO, dwCurrentFrame)
REPLAY_OFFSET(frame_grab_info, NVFBC_FRAME_GRAB_INFO, bIsNewFrame)
REPLAY_OFFSET(frame_grab_info, NVFBC_FRAME_GRAB_INFO, ulTimestampUs)
REPLAY_OFFSET(frame_grab_info, NVFBC_FRAME_GRAB_INFO, dwMissedFrames)
REPLAY_OFFSET(frame_grab_info, NVFBC_FRAME_GRAB_INFO,
              bRequiredPostProcessing)
REPLAY_OFFSET(frame_grab_info, NVFBC_FRAME_GRAB_INFO, bDirectCapture)
REPLAY_OFFSET(frame_grab_info, NVFBC_FRAME_GRAB_INFO, bCursorVisible)
REPLAY_OFFSET(frame_grab_info, NVFBC_FRAME_GRAB_INFO, bCursorComposited)

REPLAY_LAYOUT(create_handle_params, NVFBC_CREATE_HANDLE_PARAMS)
REPLAY_OFFSET(create_handle_params, NVFBC_CREATE_HANDLE_PARAMS, dwVersion)
REPLAY_OFFSET(create_handle_params, NVFBC_CREATE_HANDLE_PARAMS, privateData)
REPLAY_OFFSET(create_handle_params, NVFBC_CREATE_HANDLE_PARAMS,
              privateDataSize)
REPLAY_OFFSET(create_handle_params, NVFBC_CREATE_HANDLE_PARAMS,
              bExternallyManagedContext)
REPLAY_OFFSET(create_handle_params, NVFBC_CREATE_HANDLE_PARAMS, glxCtx)
REPLAY_OFFSET(create_handle_params, NVFBC_CREATE_HANDLE_PARAMS, glxFBConfig)
REPLAY_OFFSET(create_handle_params, NVFBC_CREATE_HANDLE_PARAMS, bUseEGL)
REPLAY_OFFSET(create_handle_params, NVFBC_CREATE_HANDLE_PARAMS, eBackend)
REPLAY_OFFSET(create_handle_params, NVFBC_CREATE_HANDLE_PARAMS,
              portalRestoreToken)

REPLAY_LAYOUT(destroy_handle_params, NVFBC_DESTROY_HANDLE_PARAMS)
REPLAY_OFFSET(destroy_handle_params, NVFBC_DESTROY_HANDLE_PARAMS, dwVersion)

REPLAY_LAYOUT(get_status_params, NVFBC_GET_STATUS_PARAMS)
REPLAY_OFFSET(get_status_params, NVFBC_GET_STATUS_PARAMS, dwVersion)
REPLAY_OFFSET(get_status_params, NVFBC_GET_STATUS_PARAMS,
              bIsCapturePossible)
REPLAY_OFFSET(get_status_params, NVFBC_GET_STATUS_PARAMS,
              bCurrentlyCapturing)
REPLAY_OFFSET(get_status_params, NVFBC_GET_STATUS_PARAMS, bCanCreateNow)
REPLAY_OFFSET(get_status_params, NVFBC_GET_STATUS_PARAMS, screenSize)
REPLAY_OFFSET(get_status_params, NVFBC_GET_STATUS_PARAMS, bXRandRAvailable)
REPLAY_OFFSET(get_status_params, NVFBC_GET_STATUS_PARAMS, outputs)
REPLAY_OFFSET(get_status_params, NVFBC_GET_STATUS_PARAMS, dwOutputNum)
REPLAY_OFFSET(get_status_params, NVFBC_GET_STATUS_PARAMS, dwNvFBCVersion)
REPLAY_OFFSET(get_status_params, NVFBC_GET_STATUS_PARAMS, bInModeset)
REPLAY_OFFSET(get_status_params, NVFBC_GET_STATUS_PARAMS,
              portalRestoreToken)
REPLAY_OFFSET(get_status_params, NVFBC_GET_STATUS_PARAMS, dwPid)
REPLAY_OFFSET(get_status_params, NVFBC_GET_STATUS_PARAMS, dwDbusTimeoutMs)
REPLAY_OFFSET(get_status_params, NVFBC_GET_STATUS_PARAMS,
              dwCaptureTargetCount)
REPLAY_OFFSET(get_status_params, NVFBC_GET_STATUS_PARAMS,
              captureTargetSizes)

REPLAY_LAYOUT(create_capture_session_params,
              NVFBC_CREATE_CAPTURE_SESSION_PARAMS)
REPLAY_OFFSET(create_capture_session_params,
              NVFBC_CREATE_CAPTURE_SESSION_PARAMS, dwVersion)
REPLAY_OFFSET(create_capture_session_params,
              NVFBC_CREATE_CAPTURE_SESSION_PARAMS, eCaptureType)
REPLAY_OFFSET(create_capture_session_params,
              NVFBC_CREATE_CAPTURE_SESSION_PARAMS, eTrackingType)
REPLAY_OFFSET(create_capture_session_params,
              NVFBC_CREATE_CAPTURE_SESSION_PARAMS, dwOutputId)
REPLAY_OFFSET(create_capture_session_params,
              NVFBC_CREATE_CAPTURE_SESSION_PARAMS, captureBox)
REPLAY_OFFSET(create_capture_session_params,
              NVFBC_CREATE_CAPTURE_SESSION_PARAMS, frameSize)
REPLAY_OFFSET(create_capture_session_params,
              NVFBC_CREATE_CAPTURE_SESSION_PARAMS, bWithCursor)
REPLAY_OFFSET(create_capture_session_params,
              NVFBC_CREATE_CAPTURE_SESSION_PARAMS,
              bDisableAutoModesetRecovery)
REPLAY_OFFSET(create_capture_session_params,
              NVFBC_CREATE_CAPTURE_SESSION_PARAMS, bRoundFrameSize)
REPLAY_OFFSET(create_capture_session_params,
              NVFBC_CREATE_CAPTURE_SESSION_PARAMS, dwSamplingRateMs)
REPLAY_OFFSET(create_capture_session_params,
              NVFBC_CREATE_CAPTURE_SESSION_PARAMS, bPushModel)
REPLAY_OFFSET(create_capture_session_params,
              NVFBC_CREATE_CAPTURE_SESSION_PARAMS, bAllowDirectCapture)
REPLAY_OFFSET(create_capture_session_params,
              NVFBC_CREATE_CAPTURE_SESSION_PARAMS, dwPid)
REPLAY_OFFSET(create_capture_session_params,
              NVFBC_CREATE_CAPTURE_SESSION_PARAMS, dwDbusTimeoutMs)
REPLAY_OFFSET(create_capture_session_params,
              NVFBC_CREATE_CAPTURE_SESSION_PARAMS, dwCaptureTarget)

REPLAY_LAYOUT(destroy_capture_session_params,
              NVFBC_DESTROY_CAPTURE_SESSION_PARAMS)
REPLAY_OFFSET(destroy_capture_session_params,
              NVFBC_DESTROY_CAPTURE_SESSION_PARAMS, dwVersion)

REPLAY_LAYOUT(bind_context_params, NVFBC_BIND_CONTEXT_PARAMS)
REPLAY_OFFSET(bind_context_params, NVFBC_BIND_CONTEXT_PARAMS, dwVersion)

REPLAY_LAYOUT(release_context_params, NVFBC_RELEASE_CONTEXT_PARAMS)
REPLAY_OFFSET(release_context_params, NVFBC_RELEASE_CONTEXT_PARAMS, dwVersion)

REPLAY_LAYOUT(tosys_setup_params, NVFBC_TOSYS_SETUP_PARAMS)
REPLAY_LAYOUT(tosys_grab_frame_params, NVFBC_TOSYS_GRAB_FRAME_PARAMS)

REPLAY_LAYOUT(tocuda_setup_params, NVFBC_TOCUDA_SETUP_PARAMS)
REPLAY_OFFSET(tocuda_setup_params, NVFBC_TOCUDA_SETUP_PARAMS, dwVersion)
REPLAY_OFFSET(tocuda_setup_params, NVFBC_TOCUDA_SETUP_PARAMS, eBufferFormat)

size_t replay_nvfbc_sizeof_tocuda_grab_frame_params(void) {
    return sizeof(NVFBC_TOCUDA_GRAB_FRAME_PARAMS);
}
size_t replay_nvfbc_alignof_tocuda_grab_frame_params(void) {
    return _Alignof(NVFBC_TOCUDA_GRAB_FRAME_PARAMS);
}
REPLAY_OFFSET(tocuda_grab_frame_params, NVFBC_TOCUDA_GRAB_FRAME_PARAMS,
              dwVersion)
REPLAY_OFFSET(tocuda_grab_frame_params, NVFBC_TOCUDA_GRAB_FRAME_PARAMS,
              dwFlags)
REPLAY_OFFSET(tocuda_grab_frame_params, NVFBC_TOCUDA_GRAB_FRAME_PARAMS,
              pCUDADeviceBuffer)
REPLAY_OFFSET(tocuda_grab_frame_params, NVFBC_TOCUDA_GRAB_FRAME_PARAMS,
              pFrameGrabInfo)
REPLAY_OFFSET(tocuda_grab_frame_params, NVFBC_TOCUDA_GRAB_FRAME_PARAMS,
              dwTimeoutMs)

REPLAY_LAYOUT(togl_setup_params, NVFBC_TOGL_SETUP_PARAMS)
REPLAY_LAYOUT(togl_grab_frame_params, NVFBC_TOGL_GRAB_FRAME_PARAMS)

REPLAY_LAYOUT(composite_cursor_params, NVFBC_COMPOSITE_CURSOR_PARAMS)
REPLAY_OFFSET(composite_cursor_params, NVFBC_COMPOSITE_CURSOR_PARAMS,
              dwVersion)
REPLAY_OFFSET(composite_cursor_params, NVFBC_COMPOSITE_CURSOR_PARAMS,
              bWithCursor)

size_t replay_nvfbc_sizeof_api_function_list(void) {
    return sizeof(NVFBC_API_FUNCTION_LIST);
}
size_t replay_nvfbc_alignof_api_function_list(void) {
    return _Alignof(NVFBC_API_FUNCTION_LIST);
}
REPLAY_OFFSET(api_function_list, NVFBC_API_FUNCTION_LIST, dwVersion)
REPLAY_OFFSET(api_function_list, NVFBC_API_FUNCTION_LIST,
              nvFBCGetLastErrorStr)
REPLAY_OFFSET(api_function_list, NVFBC_API_FUNCTION_LIST, nvFBCCreateHandle)
REPLAY_OFFSET(api_function_list, NVFBC_API_FUNCTION_LIST, nvFBCDestroyHandle)
REPLAY_OFFSET(api_function_list, NVFBC_API_FUNCTION_LIST, nvFBCGetStatus)
REPLAY_OFFSET(api_function_list, NVFBC_API_FUNCTION_LIST,
              nvFBCCreateCaptureSession)
REPLAY_OFFSET(api_function_list, NVFBC_API_FUNCTION_LIST,
              nvFBCDestroyCaptureSession)
REPLAY_OFFSET(api_function_list, NVFBC_API_FUNCTION_LIST, nvFBCToSysSetUp)
REPLAY_OFFSET(api_function_list, NVFBC_API_FUNCTION_LIST,
              nvFBCToSysGrabFrame)
REPLAY_OFFSET(api_function_list, NVFBC_API_FUNCTION_LIST, nvFBCToCudaSetUp)
REPLAY_OFFSET(api_function_list, NVFBC_API_FUNCTION_LIST,
              nvFBCToCudaGrabFrame)
REPLAY_OFFSET(api_function_list, NVFBC_API_FUNCTION_LIST, pad1)
REPLAY_OFFSET(api_function_list, NVFBC_API_FUNCTION_LIST, pad2)
REPLAY_OFFSET(api_function_list, NVFBC_API_FUNCTION_LIST, pad3)
REPLAY_OFFSET(api_function_list, NVFBC_API_FUNCTION_LIST, nvFBCBindContext)
REPLAY_OFFSET(api_function_list, NVFBC_API_FUNCTION_LIST,
              nvFBCReleaseContext)
REPLAY_OFFSET(api_function_list, NVFBC_API_FUNCTION_LIST, pad4)
REPLAY_OFFSET(api_function_list, NVFBC_API_FUNCTION_LIST, pad5)
REPLAY_OFFSET(api_function_list, NVFBC_API_FUNCTION_LIST, pad6)
REPLAY_OFFSET(api_function_list, NVFBC_API_FUNCTION_LIST, pad7)
REPLAY_OFFSET(api_function_list, NVFBC_API_FUNCTION_LIST, nvFBCToGLSetUp)
REPLAY_OFFSET(api_function_list, NVFBC_API_FUNCTION_LIST,
              nvFBCToGLGrabFrame)
REPLAY_OFFSET(api_function_list, NVFBC_API_FUNCTION_LIST,
              nvFBCCompositeCursor)

size_t replay_nvfbc_sizeof_cuda_context(void) { return sizeof(CUcontext); }
size_t replay_nvfbc_alignof_cuda_context(void) { return _Alignof(CUcontext); }
size_t replay_nvfbc_sizeof_cuda_device_pointer(void) {
    return sizeof(CUdeviceptr);
}
size_t replay_nvfbc_alignof_cuda_device_pointer(void) {
    return _Alignof(CUdeviceptr);
}
size_t replay_nvfbc_sizeof_cuda_device(void) { return sizeof(CUdevice); }
size_t replay_nvfbc_alignof_cuda_device(void) { return _Alignof(CUdevice); }
size_t replay_nvfbc_sizeof_cuda_uuid(void) { return sizeof(CUuuid); }
size_t replay_nvfbc_alignof_cuda_uuid(void) { return _Alignof(CUuuid); }
size_t replay_nvfbc_sizeof_cuda_result(void) { return sizeof(CUresult); }
size_t replay_nvfbc_alignof_cuda_result(void) { return _Alignof(CUresult); }

unsigned int replay_nvfbc_api_version_major(void) {
    return (unsigned int)NVFBC_VERSION_MAJOR;
}
unsigned int replay_nvfbc_api_version_minor(void) {
    return (unsigned int)NVFBC_VERSION_MINOR;
}
unsigned int replay_nvfbc_api_version(void) {
    return (unsigned int)NVFBC_VERSION;
}

REPLAY_NVFBC_U32(ver_create_handle_params, NVFBC_CREATE_HANDLE_PARAMS_VER)
REPLAY_NVFBC_U32(ver_destroy_handle_params, NVFBC_DESTROY_HANDLE_PARAMS_VER)
REPLAY_NVFBC_U32(ver_get_status_params, NVFBC_GET_STATUS_PARAMS_VER)
REPLAY_NVFBC_U32(ver_create_capture_session_params,
                 NVFBC_CREATE_CAPTURE_SESSION_PARAMS_VER)
REPLAY_NVFBC_U32(ver_destroy_capture_session_params,
                 NVFBC_DESTROY_CAPTURE_SESSION_PARAMS_VER)
REPLAY_NVFBC_U32(ver_bind_context_params, NVFBC_BIND_CONTEXT_PARAMS_VER)
REPLAY_NVFBC_U32(ver_release_context_params, NVFBC_RELEASE_CONTEXT_PARAMS_VER)
REPLAY_NVFBC_U32(ver_tosys_setup_params, NVFBC_TOSYS_SETUP_PARAMS_VER)
REPLAY_NVFBC_U32(ver_tosys_grab_frame_params,
                 NVFBC_TOSYS_GRAB_FRAME_PARAMS_VER)
REPLAY_NVFBC_U32(ver_tocuda_setup_params, NVFBC_TOCUDA_SETUP_PARAMS_VER)
REPLAY_NVFBC_U32(ver_tocuda_grab_frame_params,
                 NVFBC_TOCUDA_GRAB_FRAME_PARAMS_VER)
REPLAY_NVFBC_U32(ver_togl_setup_params, NVFBC_TOGL_SETUP_PARAMS_VER)
REPLAY_NVFBC_U32(ver_togl_grab_frame_params,
                 NVFBC_TOGL_GRAB_FRAME_PARAMS_VER)
REPLAY_NVFBC_U32(ver_composite_cursor_params,
                 NVFBC_COMPOSITE_CURSOR_PARAMS_VER)

REPLAY_NVFBC_U32(bool_false, NVFBC_FALSE)
REPLAY_NVFBC_U32(bool_true, NVFBC_TRUE)

REPLAY_NVFBC_U32(status_success, NVFBC_SUCCESS)
REPLAY_NVFBC_U32(status_err_api_version, NVFBC_ERR_API_VERSION)
REPLAY_NVFBC_U32(status_err_internal, NVFBC_ERR_INTERNAL)
REPLAY_NVFBC_U32(status_err_invalid_param, NVFBC_ERR_INVALID_PARAM)
REPLAY_NVFBC_U32(status_err_invalid_ptr, NVFBC_ERR_INVALID_PTR)
REPLAY_NVFBC_U32(status_err_invalid_handle, NVFBC_ERR_INVALID_HANDLE)
REPLAY_NVFBC_U32(status_err_max_clients, NVFBC_ERR_MAX_CLIENTS)
REPLAY_NVFBC_U32(status_err_unsupported, NVFBC_ERR_UNSUPPORTED)
REPLAY_NVFBC_U32(status_err_out_of_memory, NVFBC_ERR_OUT_OF_MEMORY)
REPLAY_NVFBC_U32(status_err_bad_request, NVFBC_ERR_BAD_REQUEST)
REPLAY_NVFBC_U32(status_err_x, NVFBC_ERR_X)
REPLAY_NVFBC_U32(status_err_glx, NVFBC_ERR_GLX)
REPLAY_NVFBC_U32(status_err_gl, NVFBC_ERR_GL)
REPLAY_NVFBC_U32(status_err_cuda, NVFBC_ERR_CUDA)
REPLAY_NVFBC_U32(status_err_encoder, NVFBC_ERR_ENCODER)
REPLAY_NVFBC_U32(status_err_context, NVFBC_ERR_CONTEXT)
REPLAY_NVFBC_U32(status_err_must_recreate, NVFBC_ERR_MUST_RECREATE)
REPLAY_NVFBC_U32(status_err_vulkan, NVFBC_ERR_VULKAN)
REPLAY_NVFBC_U32(status_err_egl, NVFBC_ERR_EGL)
REPLAY_NVFBC_U32(status_err_dbus, NVFBC_ERR_DBUS)
REPLAY_NVFBC_U32(status_err_pipewire, NVFBC_ERR_PIPEWIRE)
REPLAY_NVFBC_U32(status_err_drm, NVFBC_ERR_DRM)

REPLAY_NVFBC_U32(capture_to_sys, NVFBC_CAPTURE_TO_SYS)
REPLAY_NVFBC_U32(capture_shared_cuda, NVFBC_CAPTURE_SHARED_CUDA)
REPLAY_NVFBC_U32(capture_to_gl, NVFBC_CAPTURE_TO_GL)

REPLAY_NVFBC_U32(tracking_default, NVFBC_TRACKING_DEFAULT)
REPLAY_NVFBC_U32(tracking_output, NVFBC_TRACKING_OUTPUT)
REPLAY_NVFBC_U32(tracking_screen, NVFBC_TRACKING_SCREEN)

REPLAY_NVFBC_U32(buffer_format_argb, NVFBC_BUFFER_FORMAT_ARGB)
REPLAY_NVFBC_U32(buffer_format_rgb, NVFBC_BUFFER_FORMAT_RGB)
REPLAY_NVFBC_U32(buffer_format_nv12, NVFBC_BUFFER_FORMAT_NV12)
REPLAY_NVFBC_U32(buffer_format_yuv444p, NVFBC_BUFFER_FORMAT_YUV444P)
REPLAY_NVFBC_U32(buffer_format_rgba, NVFBC_BUFFER_FORMAT_RGBA)
REPLAY_NVFBC_U32(buffer_format_bgra, NVFBC_BUFFER_FORMAT_BGRA)

REPLAY_NVFBC_U32(backend_auto, NVFBC_BACKEND_AUTO)
REPLAY_NVFBC_U32(backend_x11, NVFBC_BACKEND_X11)
REPLAY_NVFBC_U32(backend_pipewire, NVFBC_BACKEND_PIPEWIRE)
REPLAY_NVFBC_U32(backend_direct, NVFBC_BACKEND_DIRECT)

REPLAY_NVFBC_U32(tocuda_grab_flags_noflags,
                 NVFBC_TOCUDA_GRAB_FLAGS_NOFLAGS)
REPLAY_NVFBC_U32(tocuda_grab_flags_nowait, NVFBC_TOCUDA_GRAB_FLAGS_NOWAIT)
REPLAY_NVFBC_U32(tocuda_grab_flags_force_refresh,
                 NVFBC_TOCUDA_GRAB_FLAGS_FORCE_REFRESH)
REPLAY_NVFBC_U32(tocuda_grab_flags_nowait_if_new_frame_ready,
                 NVFBC_TOCUDA_GRAB_FLAGS_NOWAIT_IF_NEW_FRAME_READY)

REPLAY_NVFBC_U32(err_str_len, NVFBC_ERR_STR_LEN)
REPLAY_NVFBC_U32(output_max, NVFBC_OUTPUT_MAX)
REPLAY_NVFBC_U32(output_name_len, NVFBC_OUTPUT_NAME_LEN)
REPLAY_NVFBC_U32(portal_restore_token_len,
                 NVFBC_PORTAL_RESTORE_TOKEN_LEN)
REPLAY_NVFBC_U32(direct_max_capture_targets,
                 NVFBC_DIRECT_MAX_CAPTURE_TARGETS)

REPLAY_CUDA_U32(header_version, CUDA_VERSION)
REPLAY_CUDA_U32(result_success, CUDA_SUCCESS)
REPLAY_CUDA_U32(get_proc_address_legacy_stream,
                CU_GET_PROC_ADDRESS_LEGACY_STREAM)
REPLAY_CUDA_U32(pointer_attribute_context, CU_POINTER_ATTRIBUTE_CONTEXT)
REPLAY_CUDA_U32(pointer_attribute_memory_type,
                CU_POINTER_ATTRIBUTE_MEMORY_TYPE)
REPLAY_CUDA_U32(pointer_attribute_device_pointer,
                CU_POINTER_ATTRIBUTE_DEVICE_POINTER)
REPLAY_CUDA_U32(pointer_attribute_host_pointer,
                CU_POINTER_ATTRIBUTE_HOST_POINTER)
REPLAY_CUDA_U32(pointer_attribute_sync_memops,
                CU_POINTER_ATTRIBUTE_SYNC_MEMOPS)
REPLAY_CUDA_U32(pointer_attribute_buffer_id, CU_POINTER_ATTRIBUTE_BUFFER_ID)
REPLAY_CUDA_U32(pointer_attribute_is_managed,
                CU_POINTER_ATTRIBUTE_IS_MANAGED)
REPLAY_CUDA_U32(pointer_attribute_device_ordinal,
                CU_POINTER_ATTRIBUTE_DEVICE_ORDINAL)
REPLAY_CUDA_U32(pointer_attribute_range_start_addr,
                CU_POINTER_ATTRIBUTE_RANGE_START_ADDR)
REPLAY_CUDA_U32(pointer_attribute_range_size,
                CU_POINTER_ATTRIBUTE_RANGE_SIZE)
REPLAY_CUDA_U32(pointer_attribute_mapped, CU_POINTER_ATTRIBUTE_MAPPED)
REPLAY_CUDA_U32(memory_type_host, CU_MEMORYTYPE_HOST)
REPLAY_CUDA_U32(memory_type_device, CU_MEMORYTYPE_DEVICE)
REPLAY_CUDA_U32(memory_type_array, CU_MEMORYTYPE_ARRAY)
REPLAY_CUDA_U32(memory_type_unified, CU_MEMORYTYPE_UNIFIED)
REPLAY_CUDA_U32(ctx_sched_auto, CU_CTX_SCHED_AUTO)
REPLAY_CUDA_U32(ctx_map_host, CU_CTX_MAP_HOST)
REPLAY_CUDA_U32(ctx_sync_memops, CU_CTX_SYNC_MEMOPS)

REPLAY_CUDA_U32(abi_version_get_proc_address, 12000)
REPLAY_CUDA_U32(abi_version_init, 2000)
REPLAY_CUDA_U32(abi_version_driver_get_version, 2020)
REPLAY_CUDA_U32(abi_version_device_get_by_pci_bus_id, 4010)
REPLAY_CUDA_U32(abi_version_device_get_pci_bus_id, 4010)
REPLAY_CUDA_U32(abi_version_device_get_uuid_v2, 11040)
REPLAY_CUDA_U32(abi_version_ctx_create_v2, 3020)
REPLAY_CUDA_U32(abi_version_ctx_get_current, 4000)
REPLAY_CUDA_U32(abi_version_ctx_get_device, 2000)
REPLAY_CUDA_U32(abi_version_pointer_get_attribute, 4000)
REPLAY_CUDA_U32(abi_version_ctx_destroy_v2, 4000)
REPLAY_CUDA_U32(abi_version_mem_alloc_v2, 3020)
REPLAY_CUDA_U32(abi_version_memcpy_dtod_v2, 3020)
REPLAY_CUDA_U32(abi_version_ctx_synchronize, 2000)
REPLAY_CUDA_U32(abi_version_mem_free_v2, 3020)

unsigned int replay_nvfbc_contract_mask(void) { return 0x1fffu; }
unsigned int replay_cuda_signature_mask(void) { return 0x7fffu; }
size_t replay_nvfbc_offset_api_composite_cursor(void) {
    return offsetof(NVFBC_API_FUNCTION_LIST, nvFBCCompositeCursor);
}
unsigned int replay_nvfbc_cuda_signature_mask(void) { return 0x7fffu; }
