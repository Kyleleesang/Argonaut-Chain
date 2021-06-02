use codec::{Decode, Encode};
use frame_support::{
	decl_event, decl_module, decl_storage,
	dispatch::{DispatchResult, Vec},
	ensure,
};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_core::{
	crypto::Public as _,
	H256,
	H512,
	sr25519::{Public, Signature},
};
use sp_std::collections::btree_map::BTreeMap;
use sp_runtime::{
	traits::{BlakeTwo256, Hash, SaturatedConversion},
	transaction_validity::{TransactionLongevity, ValidTransaction},
};
use super::{block_author::BlockAuthor, issuance::Issuance};

pub trait Trait: frame_system::Trait {
	/// The ubiquitous Event type
	type Event: From<Event> + Into<<Self as frame_system::Trait>::Event>;

	/// A source to determine the block author
	type BlockAuthor: BlockAuthor;

	/// A source to determine the issuance portion of the block reward
	type Issuance: Issuance<<Self as frame_system::Trait>::BlockNumber, Value>;
}

pub type Value = u128;

/// Single transaction to be dispatched
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(PartialEq, Eq, PartialOrd, Ord, Default, Clone, Encode, Decode, Hash, Debug)]
pub struct Transaction {
	/// UTXOs to be used as inputs for current transaction
	pub inputs: Vec<TransactionInput>,

	/// UTXOs to be created as a result of current transaction dispatch
	pub outputs: Vec<TransactionOutput>,
}

/// Single transaction input that refers to one UTXO
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(PartialEq, Eq, PartialOrd, Ord, Default, Clone, Encode, Decode, Hash, Debug)]
pub struct TransactionInput {
	/// Reference to an UTXO to be spent
	pub outpoint: H256,

	/// Proof that transaction owner is authorized to spend referred UTXO &
	/// that the entire transaction is untampered
	pub sigscript: H512,
}

/// Single transaction output to create upon transaction dispatch
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(PartialEq, Eq, PartialOrd, Ord, Default, Clone, Encode, Decode, Hash, Debug)]
pub struct TransactionOutput {
	/// Value associated with this output
	pub value: Value,

	/// Public key associated with this output. In order to spend this output
	/// owner must provide a proof by hashing the whole `Transaction` and
	/// signing it with a corresponding private key.
	pub pubkey: H256,
}

decl_storage! {
	trait Store for Module<T: Trait> as Utxo {
		/// All valid unspent transaction outputs are stored in this map.
		/// Initial set of UTXO is populated from the list stored in genesis.
		/// We use the identity hasher here because the cryptographic hashing is
		/// done explicitly. TODO In the future we should remove the explicit hashing,
		/// and use blake2_128_concat here. I'm deferring that so as not to break
		/// the workshop inputs.
		UtxoStore build(|config: &GenesisConfig| {
			config.genesis_utxos
				.iter()
				.cloned()
				.map(|u| (BlakeTwo256::hash_of(&u), u))
				.collect::<Vec<_>>()
		}): map hasher(identity) H256 => Option<TransactionOutput>;

		/// Total reward value to be redistributed among authorities.
		/// It is accumulated from transactions during block execution
		/// and then dispersed to validators on block finalization.
		pub RewardTotal get(fn reward_total): Value;
	}

	add_extra_genesis {
		config(genesis_utxos): Vec<TransactionOutput>;
	}
}

// External functions: callable by the end user
decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event() = default;

		/// Dispatch a single transaction and update UTXO set accordingly
		#[weight = 1_000_000] //TODO weight should be proportional to number of inputs + outputs
		pub fn spend(_origin, transaction: Transaction) -> DispatchResult {
									// TransactionValidity{}
			let transaction_validity = Self::validate_transaction(&transaction)?;
			ensure!(transaction_validity.requires.is_empty(), "missing inputs");

			Self::update_storage(&transaction, transaction_validity.priority as Value)?;

			Self::deposit_event(Event::TransactionSuccess(transaction));

			Ok(())
		}

		/// Handler called by the system on block finalization
		fn on_finalize() {
			match T::BlockAuthor::block_author() {
				// Block author did not provide key to claim reward
				None => Self::deposit_event(Event::RewardsWasted),
				// Block author did provide key, so issue thir reward
				Some(author) => Self::disperse_reward(&author),
			}
		}
	}
}

decl_event!(
	pub enum Event {
		/// Transaction was executed successfully
		TransactionSuccess(Transaction),
		/// Rewards were issued. Amount, UTXO hash.
		RewardsIssued(Value, H256),
		/// Rewards were wasted
		RewardsWasted,
	}
);

// "Internal" functions, callable by code.
impl<T: Trait> Module<T> {

	/// Check transaction for validity, errors, & race conditions
	/// Called by both transaction pool and runtime execution
	///
	/// Ensures that:
	/// - inputs and outputs are not empty
	/// - all inputs match to existing, unspent and unlocked outputs
	/// - each input is used exactly once
	/// - each output is defined exactly once and has nonzero value
	/// - total output value must not exceed total input value
	/// - new outputs do not collide with existing ones
	/// - sum of input and output values does not overflow
	/// - provided signatures are valid
	/// - transaction outputs cannot be modified by malicious nodes
	pub fn validate_transaction(transaction: &Transaction) -> Result<ValidTransaction, &'static str> {
		// Check basic requirements
		ensure!(!transaction.inputs.is_empty(), "no inputs");
		ensure!(!transaction.outputs.is_empty(), "no outputs");

		{
			let input_set: BTreeMap<_, ()> =transaction.inputs.iter().map(|input| (input, ())).collect();
			ensure!(input_set.len() == transaction.inputs.len(), "each input must only be used once");
		}
		{
			let output_set: BTreeMap<_, ()> = transaction.outputs.iter().map(|output| (output, ())).collect();
			ensure!(output_set.len() == transaction.outputs.len(), "each output must be defined only once");
		}

		let mut total_input: Value = 0;
		let mut total_output: Value = 0;
		let mut output_index: u64 = 0;
		let simple_transaction = Self::get_simple_transaction(transaction);

		// Variables sent to transaction pool
		let mut missing_utxos = Vec::new();
		let mut new_utxos = Vec::new();
		let mut reward = 0;

		// Check that inputs are valid
		for input in transaction.inputs.iter() {
			if let Some(input_utxo) = <UtxoStore>::get(&input.outpoint) {
				ensure!(sp_io::crypto::sr25519_verify(
					&Signature::from_raw(*input.sigscript.as_fixed_bytes()),
					&simple_transaction,
					&Public::from_h256(input_utxo.pubkey)
				), "signature must be valid" );
				total_input = total_input.checked_add(input_utxo.value).ok_or("input value overflow")?;
			} else {
				missing_utxos.push(input.outpoint.clone().as_fixed_bytes().to_vec());
			}
		}

		// Check that outputs are valid
		for output in transaction.outputs.iter() {
			ensure!(output.value > 0, "output value must be nonzero");
			let hash = BlakeTwo256::hash_of(&(&transaction.encode(), output_index));
			output_index = output_index.checked_add(1).ok_or("output index overflow")?;
			ensure!(!<UtxoStore>::contains_key(hash), "output already exists");
			total_output = total_output.checked_add(output.value).ok_or("output value overflow")?;
			new_utxos.push(hash.as_fixed_bytes().to_vec());
		}

		// If no race condition, check the math
		if missing_utxos.is_empty() {
			ensure!( total_input >= total_output, "output value must not exceed input value");
			reward = total_input.checked_sub(total_output).ok_or("reward underflow")?;
		}

		// Returns transaction details
		Ok(ValidTransaction {
			requires: missing_utxos,
			provides: new_utxos,
			priority: reward as u64,
			longevity: TransactionLongevity::max_value(),
			propagate: true,
		})
	}

	/// Update storage to reflect changes made by transaction
	/// Where each utxo key is a hash of the entire transaction and its order in the TransactionOutputs vector
	fn update_storage(transaction: &Transaction, reward: Value) -> DispatchResult {
		// Calculate new reward total
		let new_total = <RewardTotal>::get()
			.checked_add(reward)
			.ok_or("Reward overflow")?;
		<RewardTotal>::put(new_total);

		// Removing spent UTXOs
		for input in &transaction.inputs {
			<UtxoStore>::remove(input.outpoint);
		}

		let mut index: u64 = 0;
		for output in &transaction.outputs {
			let hash = BlakeTwo256::hash_of(&(&transaction.encode(), index));
			index = index.checked_add(1).ok_or("output index overflow")?;
			<UtxoStore>::insert(hash, output);
		}

		Ok(())
	}

	/// Redistribute combined reward value to block Author
	fn disperse_reward(author: &Public) {
		let reward = RewardTotal::take() + T::Issuance::issuance(frame_system::Module::<T>::block_number());

		let utxo = TransactionOutput {
			value: reward,
			pubkey: H256::from_slice(author.as_slice()),
		};

		let hash = BlakeTwo256::hash_of(&(&utxo,
					<frame_system::Module<T>>::block_number().saturated_into::<u64>()));

		<UtxoStore>::insert(hash, utxo);
		Self::deposit_event(Event::RewardsIssued(reward, hash));
	}

	// Strips a transaction of its Signature fields by replacing value with ZERO-initialized fixed hash.
	pub fn get_simple_transaction(transaction: &Transaction) -> Vec<u8> {//&'a [u8] {
		let mut trx = transaction.clone();
		for input in trx.inputs.iter_mut() {
			input.sigscript = H512::zero();
		}

		trx.encode()
	}

	/// Helper fn for Transaction Pool
	/// Checks for race condition, if a certain trx is missing input_utxos in UtxoStore
	/// If None missing inputs: no race condition, gtg
	/// if Some(missing inputs): there are missing variables
	pub fn get_missing_utxos(transaction: &Transaction) -> Vec<&H256> {
		let mut missing_utxos = Vec::new();
		for input in transaction.inputs.iter() {
			if <UtxoStore>::get(&input.outpoint).is_none() {
				missing_utxos.push(&input.outpoint);
			}
		}
		missing_utxos
	}
}