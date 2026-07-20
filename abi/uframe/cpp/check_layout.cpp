// Compile-only conformance check for uframe_metadata_abi_v1.hpp
#include "uframe_metadata_abi_v1.hpp"

int main() {
    uprotocol::v2::UFrameMetadataAbiV1 m{};
    m.magic[0] = 'U';
    m.kind = static_cast<std::uint8_t>(uprotocol::v2::UFrameKind::Publish);
    (void)m;
    return 0;
}
