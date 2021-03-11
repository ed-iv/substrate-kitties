#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
// Althought we don't use them directly, we need to include StorageDoubleMap and StorageValue
// so that we can access them via their getters.
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, dispatch::DispatchResult,
    traits::Randomness, RuntimeDebug, RuntimeDebug, StorageDoubleMap, StorageValue,
};
use frame_system::ensure_signed;
use sp_io::hashing::blake2_128;

// RuntimeDebug is just like Debug in native build, but becomes simplified version
// in wasm build.
#[derive(Encode, Decode, Clone, RuntimeDebug, PartialEq, Eq)]
pub struct Kitty(pub [u8; 16]);

// Inherits from --vvvvvvvvvvvvvvvv
pub trait Trait: frame_system::Trait {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
}

decl_event! {
    pub enum Event<T> where
        <T as frame_system::Trait>::AccountId,
    {
        // A kitty is created. \[owner, kitty_id, kitty\]
        KittyCreated(AccountId, u32, Kitty),
    }
}

decl_error! {
    pub enum Error for Module<T: Trait> {
        KittiesIdOverflow,
    }
}

decl_storage! {
    // Must be diff for each module ----vvvvvvv otherwise there could
    // be storage collisions.
    //
    // Note: each declared storage eventually becomes a struct when macro
    // is expanded. This is why we need to declare pub visibility, so that
    // the struct is also pub.
    trait Store for Module<T: Trait> as Kitties {
        // Stores all the kitties, key is kitty_id
        pub Kitties get(fn kitties): double_map
            hasher(blake2_128_concat) T::AccountId,
            hasher(blake2_128_concat) u32 => Option<Kitty>;

        // Stores the next kitty id
        pub NextKittyId get(fn next_kitty_id): u32;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;

        fn deposit_event() = default;

        #[weight = 1000]
        pub fn create(origin) {
            // Reminder: '?' is error propagation operator. If value in Result
            // is Ok(v) it unwraps v and returns it. If value in Result is Err(err)
            // the function will return the error to the calling code. Note, that the
            // error is also converted to the type specified in Result of current function
            // using From trait.
            let sender = ensure_signed(origin)?;

            // If returned DispatchResult is OK, the mutation is committed. Otherwise
            // it is discarded.
            //
            // Mutatable reference --vvvvvvv
            NextKittyId::try_mutate(|next_id| -> DispatchResult {
                let current_id = *next_id;
                *next_id = next_id.checked_add(1).ok_or(<Error<T>>::KittiesIdOverflow)?;


                // Generate random 128bit value to use as kitty DNA.
                let payload = (
                    <pallet_randomness_collective_flip::Module<T> as Randomness<T::Hash>>::random_seed(),
                    &sender,
                    <frame_system::Module<T>>::extrinsic_index(),
                    "foo",
                );
                let dna = payload.using_encoded(blake2_128);

                // Create and store kitty
                let kitty = Kitty(dna);
                Kitties::<T>::insert(&sender, current_id, kitty.clone());
                // <Kitties<T>>::insert(&sender, current_id, kitty.clone());

                // Emit event
                Self::deposit_event(RawEvent::KittyCreated(sender, current_id, kitty));

                Ok(())
            })?;


        }
    }
}
