#![cfg_attr(not(feature = "std"), no_std)]
// some of the basic functions to add
use sp_core::offchain::{
  Duration, IpfsRequest, IpfsResponse, OpaqueMultiaddr, Timestamp
};
// need this for native
use sp_runtime::offchain::ipfs;


/// The pallet's configuration trait.
pub trait ArgonautIPFS: system::Trait { // Use traits here to tightly couple to runtime
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}


impl<T: Trait> Module<T> {
    // "Sends" a request to the local IPFS node by adding it to the offchain storage
    fn ipfs_request(req: IpfsRequest, deadline: impl Into<Option<Timestamp>>)
      -> Result<IpfsResponse, Error<T>>

    // Reads from the `ConnectionQueue` and connects / disconnects
    // from desired / undesired peers, respectively
    fn connection_housekeeping() -> Result<(), Error<T>>

    // Reads `FindPeer` and `GetProviders` commands from the `DhtQueue`,
    // and requests their execution from the native runtime
    fn handle_dht_requests() -> Result<(), Error<T>>

    // Reads `AddBytes`, `CatBytes`, `DataCommand`, `RemoveBlock`, `InsertPin`,
    // and `RemovePin` commands from the `DataQueue` and requests their
    // execution from the native runtime.
    fn handle_data_requests() -> Result<(), Error<T>>

    // Logs metadata (the number of connected peers) to the console at the DEBUG log level
    fn print_metadata() -> Result<(), Error<T>>

// Commands involved in peer-to-peer connections
enum ConnectionCommand {
    ConnectTo(OpaqueMultiaddr),
    DisconnectFrom(OpaqueMultiaddr),
}

// Commands that add, remove, pin, unpin, and output data
enum DataCommand {
    AddBytes(Vec<u8>),
    CatBytes(Vec<u8>),
    InsertPin(Vec<u8>),
    RemoveBlock(Vec<u8>),
    RemovePin(Vec<u8>),
}

// Commands that query the distributed hash table (DHT)
// for peers and content
enum DhtCommand {
    FindPeer(Vec<u8>),
    GetProviders(Vec<u8>),
}

decl_event!(
    pub enum Event<T> where AccountId = <T as system::Trait>::AccountId {
        ConnectionRequested(AccountId),
        DisconnectRequested(AccountId),
        QueuedDataToAdd(AccountId),
        QueuedDataToCat(AccountId),
        QueuedDataToPin(AccountId),
        QueuedDataToRemove(AccountId),
        QueuedDataToUnpin(AccountId),
        FindPeerIssued(AccountId),
        FindProvidersIssued(AccountId),
    }
);

decl_storage! {
    // put a storage map of all the ipfs hashes where they can be found
    Hashes: map Vec<u8> => (T::AccountID, T::BlockNumber);
}


// The pallet's dispatchable functions.
decl_module! {
    /// The module declaration.
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        // Called at the beginning of every block before any extrinsics. Clears
        // `ConnectionQueue` and `DhtQueue` values every block, and clears
        // `DataQueue` every other block, since they should have been processed
        // Returns a weight of 0
        fn on_initialize(block_number: T::BlockNumber) -> Weight

        // Called at the beginning of every block to create extrinsics.
        // - `connection_housekeeping` and `handle_dht_requests` called every block
        // - `handle_data_requests` is called on every other block
        // - `print_metadata` is called every 5 blocks
        // blocks to alleviate some bandwidth and storage congestion
        fn offchain_worker(block_number: T::BlockNumber)

        /// Mark a `Multiaddr` as a desired connection target.
        #[weight = 100_000]
        pub fn ipfs_connect(origin, addr: Vec<u8>)

        /// Queues a `Multiaddr` to be disconnected
        #[weight = 500_000]
        pub fn ipfs_disconnect(origin, addr: Vec<u8>)

        /// Add arbitrary bytes to the IPFS repository.
        #[weight = 200_000]
        pub fn ipfs_add_bytes(origin, data: Vec<u8>)

        /// Find and output IPFS data pointed to by the given `Cid`
        #[weight = 100_000]
        pub fn ipfs_cat_bytes(origin, cid: Vec<u8>)

        /// Remove bytes from IPFS by `Cid`
        #[weight = 300_000]
        pub fn ipfs_remove_block(origin, cid: Vec<u8>)

        /// Pins a given `Cid` non-recursively.
        #[weight = 100_000]
        pub fn ipfs_insert_pin(origin, cid: Vec<u8>)

        /// Unpins a given `Cid` non-recursively.
        #[weight = 100_000]
        pub fn ipfs_remove_pin(origin, cid: Vec<u8>)

        /// Find addresses associated with the given `PeerId`.
        #[weight = 100_000]
        pub fn ipfs_dht_find_peer(origin, peer_id: Vec<u8>)

        /// Find the list of `PeerId`s known to be hosting the given `Cid`.
        #[weight = 100_000]
        pub fn ipfs_dht_find_providers(origin, cid: Vec<u8>)
    }
}

// The pallet's errors
decl_error! {
    pub enum Error for Module<T: Trait> {
        CantCreateRequest,
        RequestTimeout,
        RequestFailed,
    }
}