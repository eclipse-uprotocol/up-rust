/* Compile-only conformance check for uframe_metadata_abi_v1.h */
#include "uframe_metadata_abi_v1.h"

int main(void) {
    uframe_metadata_abi_v1 m = {0};
    m.magic[0] = UFRAME_ABI_MAGIC_0;
    m.kind = UFRAME_KIND_PUBLISH;
    (void)m;
    return 0;
}
