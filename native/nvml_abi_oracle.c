/*
 * Project-owned ABI comparisons for the operator-supplied NVIDIA NVML header.
 * This file intentionally contains no copied NVIDIA declarations or source.
 */
#define NVML_NO_UNVERSIONED_FUNC_DEFS 1
#include <stddef.h>
#include <nvml.h>

typedef nvmlReturn_t (*replay_nvml_init_fn)(void);
typedef nvmlReturn_t (*replay_nvml_shutdown_fn)(void);
typedef nvmlReturn_t (*replay_nvml_driver_version_fn)(char *, unsigned int);
typedef nvmlReturn_t (*replay_nvml_device_count_fn)(unsigned int *);
typedef nvmlReturn_t (*replay_nvml_device_handle_fn)(unsigned int, nvmlDevice_t *);
typedef nvmlReturn_t (*replay_nvml_device_uuid_fn)(nvmlDevice_t, char *, unsigned int);
typedef nvmlReturn_t (*replay_nvml_device_pci_fn)(nvmlDevice_t, nvmlPciInfo_t *);

#if !defined(__GNUC__) && !defined(__clang__)
#error "NVML ABI oracle requires a compiler with type-compatibility assertions"
#endif

_Static_assert(__builtin_types_compatible_p(__typeof__(&nvmlInit_v2),
                                            replay_nvml_init_fn),
               "nvmlInit_v2 signature changed");
_Static_assert(__builtin_types_compatible_p(__typeof__(&nvmlShutdown),
                                            replay_nvml_shutdown_fn),
               "nvmlShutdown signature changed");
_Static_assert(__builtin_types_compatible_p(__typeof__(&nvmlSystemGetDriverVersion),
                                            replay_nvml_driver_version_fn),
               "nvmlSystemGetDriverVersion signature changed");
_Static_assert(__builtin_types_compatible_p(__typeof__(&nvmlDeviceGetCount_v2),
                                            replay_nvml_device_count_fn),
               "nvmlDeviceGetCount_v2 signature changed");
_Static_assert(__builtin_types_compatible_p(__typeof__(&nvmlDeviceGetHandleByIndex_v2),
                                            replay_nvml_device_handle_fn),
               "nvmlDeviceGetHandleByIndex_v2 signature changed");
_Static_assert(__builtin_types_compatible_p(__typeof__(&nvmlDeviceGetUUID),
                                            replay_nvml_device_uuid_fn),
               "nvmlDeviceGetUUID signature changed");
_Static_assert(__builtin_types_compatible_p(__typeof__(&nvmlDeviceGetPciInfo_v3),
                                            replay_nvml_device_pci_fn),
               "nvmlDeviceGetPciInfo_v3 signature changed");

size_t replay_nvml_sizeof_return(void) { return sizeof(nvmlReturn_t); }
size_t replay_nvml_alignof_return(void) { return _Alignof(nvmlReturn_t); }
size_t replay_nvml_sizeof_device(void) { return sizeof(nvmlDevice_t); }
size_t replay_nvml_alignof_device(void) { return _Alignof(nvmlDevice_t); }
size_t replay_nvml_sizeof_pci_info(void) { return sizeof(nvmlPciInfo_t); }
size_t replay_nvml_alignof_pci_info(void) { return _Alignof(nvmlPciInfo_t); }
size_t replay_nvml_offset_pci_bus_id_legacy(void) {
    return offsetof(nvmlPciInfo_t, busIdLegacy);
}
size_t replay_nvml_offset_pci_domain(void) {
    return offsetof(nvmlPciInfo_t, domain);
}
size_t replay_nvml_offset_pci_bus(void) { return offsetof(nvmlPciInfo_t, bus); }
size_t replay_nvml_offset_pci_device(void) {
    return offsetof(nvmlPciInfo_t, device);
}
size_t replay_nvml_offset_pci_device_id(void) {
    return offsetof(nvmlPciInfo_t, pciDeviceId);
}
size_t replay_nvml_offset_pci_subsystem_id(void) {
    return offsetof(nvmlPciInfo_t, pciSubSystemId);
}
size_t replay_nvml_offset_pci_bus_id(void) {
    return offsetof(nvmlPciInfo_t, busId);
}
unsigned int replay_nvml_api_version(void) { return NVML_API_VERSION; }
unsigned int replay_nvml_status_success(void) { return NVML_SUCCESS; }
unsigned int replay_nvml_status_version_mismatch(void) {
    return NVML_ERROR_LIB_RM_VERSION_MISMATCH;
}
unsigned int replay_nvml_pci_legacy_buffer_size(void) {
    return NVML_DEVICE_PCI_BUS_ID_BUFFER_V2_SIZE;
}
unsigned int replay_nvml_pci_buffer_size(void) {
    return NVML_DEVICE_PCI_BUS_ID_BUFFER_SIZE;
}
unsigned int replay_nvml_uuid_buffer_size(void) {
    return NVML_DEVICE_UUID_V2_BUFFER_SIZE;
}
unsigned int replay_nvml_driver_version_buffer_size(void) {
    return NVML_SYSTEM_DRIVER_VERSION_BUFFER_SIZE;
}
unsigned int replay_nvml_signature_mask(void) { return 0x7fu; }
