/*
 * Project-owned ABI comparisons for operator-supplied NvFBC 1.8 and CUDA
 * driver headers. This file intentionally contains no copied declarations,
 * source paths, native calls, or capture implementation.
 */
#include <NvFBC.h>
#include <cuda.h>
#include <stddef.h>

_Static_assert(sizeof(CUcontext) == sizeof(void *),
               "CUDA context representation changed");
_Static_assert(sizeof(CUdeviceptr) >= sizeof(void *),
               "CUDA device pointer representation is unexpectedly narrow");
_Static_assert(
    sizeof(((NVFBC_API_FUNCTION_LIST *)0)->nvFBCCreateHandle) == sizeof(void *),
    "NvFBC create-handle function pointer representation changed");
_Static_assert(
    sizeof(((NVFBC_API_FUNCTION_LIST *)0)->nvFBCGetStatus) == sizeof(void *),
    "NvFBC status function pointer representation changed");
_Static_assert(
    sizeof(((NVFBC_API_FUNCTION_LIST *)0)->nvFBCBindContext) == sizeof(void *),
    "NvFBC bind-context function pointer representation changed");
_Static_assert(
    sizeof(((NVFBC_API_FUNCTION_LIST *)0)->nvFBCCreateCaptureSession) ==
        sizeof(void *),
    "NvFBC create-session function pointer representation changed");
_Static_assert(
    sizeof(((NVFBC_API_FUNCTION_LIST *)0)->nvFBCToCudaSetUp) == sizeof(void *),
    "NvFBC ToCUDA setup function pointer representation changed");
_Static_assert(
    sizeof(((NVFBC_API_FUNCTION_LIST *)0)->nvFBCToCudaGrabFrame) ==
        sizeof(void *),
    "NvFBC ToCUDA grab function pointer representation changed");
_Static_assert(
    sizeof(((NVFBC_API_FUNCTION_LIST *)0)->nvFBCDestroyCaptureSession) ==
        sizeof(void *),
    "NvFBC destroy-session function pointer representation changed");
_Static_assert(
    sizeof(((NVFBC_API_FUNCTION_LIST *)0)->nvFBCReleaseContext) ==
        sizeof(void *),
    "NvFBC release-context function pointer representation changed");
_Static_assert(
    sizeof(((NVFBC_API_FUNCTION_LIST *)0)->nvFBCDestroyHandle) ==
        sizeof(void *),
    "NvFBC destroy-handle function pointer representation changed");

size_t replay_nvfbc_sizeof_api_function_list(void) {
    return sizeof(NVFBC_API_FUNCTION_LIST);
}
size_t replay_nvfbc_sizeof_create_handle_params(void) {
    return sizeof(NVFBC_CREATE_HANDLE_PARAMS);
}
size_t replay_nvfbc_sizeof_get_status_params(void) {
    return sizeof(NVFBC_GET_STATUS_PARAMS);
}
size_t replay_nvfbc_sizeof_bind_context_params(void) {
    return sizeof(NVFBC_BIND_CONTEXT_PARAMS);
}
size_t replay_nvfbc_sizeof_create_capture_session_params(void) {
    return sizeof(NVFBC_CREATE_CAPTURE_SESSION_PARAMS);
}
size_t replay_nvfbc_sizeof_tocuda_setup_params(void) {
    return sizeof(NVFBC_TOCUDA_SETUP_PARAMS);
}
size_t replay_nvfbc_sizeof_tocuda_grab_frame_params(void) {
    return sizeof(NVFBC_TOCUDA_GRAB_FRAME_PARAMS);
}
size_t replay_nvfbc_sizeof_frame_grab_info(void) {
    return sizeof(NVFBC_FRAME_GRAB_INFO);
}
size_t replay_nvfbc_sizeof_destroy_capture_session_params(void) {
    return sizeof(NVFBC_DESTROY_CAPTURE_SESSION_PARAMS);
}
size_t replay_nvfbc_sizeof_release_context_params(void) {
    return sizeof(NVFBC_RELEASE_CONTEXT_PARAMS);
}
size_t replay_nvfbc_sizeof_destroy_handle_params(void) {
    return sizeof(NVFBC_DESTROY_HANDLE_PARAMS);
}
size_t replay_nvfbc_sizeof_cuda_context(void) { return sizeof(CUcontext); }
size_t replay_nvfbc_sizeof_cuda_device_pointer(void) {
    return sizeof(CUdeviceptr);
}
unsigned int replay_nvfbc_api_version(void) { return NVFBC_API_VERSION; }
unsigned int replay_nvfbc_contract_mask(void) { return 0x1fffu; }
