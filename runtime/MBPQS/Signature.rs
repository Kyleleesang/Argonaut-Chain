//Currently a WIP translating from GoLang to Rust

pub SignatureSequenceNumber: u32;

pub struct RootSignature {
    context: &Context; // Context of the MBPQS instance
    SequenceNumber: SignatureSequenceNumber;
    WOTSsig: Vec<u8> //WOTS signature of the channel root
    AuthoritzationPath: Vec<u8> // The authentication path for this signature to the rootTree root node.
    RootHash: Vec<u8> // ChannelRoot which is signed.
}
//Signature of the last OTS key in a chain tree over the next chain tree root node
pub struct GrowSignature {
    context: &Context; // Context of the MBPQS instance
    WOTSsig: Vec<u8>; //WOTS signature of the channel root
    RootHash: Vec<u8>
    chainSequenceNumber: uint32;
    ChannelID: uint32;
    layer: uint32;
}

//Signature of a message on a channel
pub struct MessageSignature {
    context: &Context; // Context of the MBPQS instance
    SequenceNumber: SignatureSequenceNumber;
    DigestValue: Vec<u8>; // Randomized value from a digest
    WOTSsig: Vec<u8>; //WOTS signature of the channel root
    AuthoritzationPath: Vec<u8> // The authentication path for this signature to the rootTree root node.
    chainSequenceNumber: uint32;
    ChannelID: uint32;
    layer:uint32;
}

impl RootSignature{
    fn GetSignedRoot(Root: &RootSignature) {
        return Root.RootHash;
    }
    fn NextAuthNode(Root: &RootSignature, previousAuthNode: Vec<u8>){
        return Root.GetSignedRoot;
    }
}

impl MessageSignature{
    fn NextAuthNode(Msg: &MessageSignature, previousAuthNode: Vec<u8>){
        if Msg.LastChainMessage{
            return previousAuthNode[0];
        } return Msg.AuthoritzationPath;
    }

    fn LastChainMessage(Msg: &MessageSignature) -> bool{
        if Msg.chainSequenceNumber == (Msg.context.chainTreeHeight(Msg.layer) - 1){
            return true;
        } return false;
    }

}