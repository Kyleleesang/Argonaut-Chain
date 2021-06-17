//Currently a WIP translating from GoLang to Rust


pub struct RootSignature {
    context: *Context; // Context of the MBPQS instance
    SequenceNumber: SignatureSequenceNumber;
    WOTSsig: byte_array::ByteArray; //WOTS signature of the channel root
    AuthoritzationPath: byte_array::ByteArray // The authentication path for this signature to the rootTree root node.
    RootHash: byte_array::ByteArray // ChannelRoot which is signed.
}
//Signature of the last OTS key in a chain tree over the next chain tree root node
pub struct GrowSignature {
    context: *Context; // Context of the MBPQS instance
    WOTSsig: byte_array::ByteArray; //WOTS signature of the channel root
    RootHash: byte_array::ByteArray;
    chainSequenceNumber: uint32;
    ChannelID: uint32;
    layer: uint32;
}

//Signature of a message on a channel
pub struct MessageSignature {
    context: *Context; // Context of the MBPQS instance
    SequenceNumber: SignatureSequenceNumber;
    DigestValue: byte_array::ByteArray; // Randomized value from a digest
    WOTSsig: byte_array::ByteArray; //WOTS signature of the channel root
    AuthoritzationPath: byte_array::ByteArray // The authentication path for this signature to the rootTree root node.
    chainSequenceNumber: uint32;
    ChannelID: uint32;
    layer:uint32;
}

impl RootSignature{
    public RootSignature
}