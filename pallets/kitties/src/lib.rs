#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
// Althought we don't use them directly, we need to include StorageDoubleMap and StorageValue
// so that we can access them via their getters.
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage,
    dispatch::{DispatchError, DispatchResult},
    ensure,
    traits::Randomness,
    RuntimeDebug, StorageDoubleMap, StorageValue,
};
use frame_system::ensure_signed;
use sp_io::hashing::blake2_128;

#[cfg(test)]
mod tests;

#[derive(Encode, Decode, Clone, Copy, RuntimeDebug, PartialEq, Eq)]
pub enum KittyGender {
    Male,
    Female,
}

// RuntimeDebug is just like Debug in native build, but becomes simplified version
// in wasm build.
#[derive(Encode, Decode, Clone, RuntimeDebug, PartialEq, Eq)]
pub struct Kitty(pub [u8; 16]);

impl Kitty {
    pub fn gender(&self) -> KittyGender {
        if self.0[0] % 2 == 0 {
            KittyGender::Male
        } else {
            KittyGender::Female
        }
    }
}

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
        // A new kitten is bred. \[owner, kitty_id, kitty\]
        KittyBred(AccountId, u32, Kitty),
    }
}

decl_error! {
    pub enum Error for Module<T: Trait> {
        KittiesIdOverflow,
        InvalidKittyId,
        SameGender,
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
            let kitty_id = Self::get_next_kitty_id()?;

            // Generate random 128bit value to use as kitty DNA.
            let dna = Self::random_value(&sender);

            // Create and store kitty
            let kitty = Kitty(dna);
            Kitties::<T>::insert(&sender, kitty_id, kitty.clone());
            // <Kitties<T>>::insert(&sender, current_id, kitty.clone());

            // Emit event
            Self::deposit_event(RawEvent::KittyCreated(sender, kitty_id, kitty));

        }

        #[weight = 1000]
        pub fn breed(origin, kitty_id_1: u32, kitty_id_2: u32) {
            let sender = ensure_signed(origin)?;
            let kitty1 = Self::kitties(&sender, kitty_id_1).ok_or(Error::<T>::InvalidKittyId)?;
            let kitty2 = Self::kitties(&sender, kitty_id_2).ok_or(Error::<T>::InvalidKittyId)?;

            // Also verifies that kitty1 and kitty2 are not the same kitty.
            ensure!(kitty1.gender() != kitty2.gender(), Error::<T>::SameGender);
            let kitty_id = Self::get_next_kitty_id()?;

            let kitty1_dna = kitty1.0;
            let kitty2_dna = kitty2.0;

            // Generate random 128bit value to use as kitty DNA.
            let selector = Self::random_value(&sender);
            let mut new_dna = [0u8; 16];

            // Combine parents and selector to create new kitty:
            for i in 0..kitty1_dna.len() {
                new_dna[i] = combine_dna(kitty1_dna[i], kitty2_dna[i], selector[i]);
            }

            let new_kitty = Kitty(new_dna);
            Kitties::<T>::insert(&sender, kitty_id, &new_kitty);

            Self::deposit_event(RawEvent::KittyBred(sender, kitty_id, new_kitty));
        }
    }
}

fn combine_dna(dna1: u8, dna2: u8, selector: u8) -> u8 {
    (!selector & dna1) | (selector & dna2)
}

impl<T: Trait> Module<T> {
    fn get_next_kitty_id() -> sp_std::result::Result<u32, DispatchError> {
        NextKittyId::try_mutate(|next_id| -> sp_std::result::Result<u32, DispatchError> {
            let current_id = *next_id;
            *next_id = next_id
                .checked_add(1)
                .ok_or(<Error<T>>::KittiesIdOverflow)?;
            Ok(current_id)
        })
    }

    fn random_value(sender: &T::AccountId) -> [u8; 16] {
        let payload = (
            <pallet_randomness_collective_flip::Module<T> as Randomness<T::Hash>>::random_seed(),
            sender,
            <frame_system::Module<T>>::extrinsic_index(),
        );
        payload.using_encoded(blake2_128)
    }
}
