use std::{convert::TryInto, fmt::{Debug, Display},};
pub use crate::core_nft::{
    crypto_hash,
    gate::{GateId, ValidGateId},
    nep178::NonFungibleTokenApprovalsReceiver,
    nep171,
    MarketApproveMsg, Payout, TokenId,
};
use near_env::{near_ext, near_log, PanicMessage};
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    collections::{LookupMap, UnorderedMap, UnorderedSet},
    env, ext_contract,
    json_types::{ValidAccountId, U128, U64},
    near_bindgen,
    serde::{Deserialize, Serialize},
    serde_json, AccountId, Balance, BorshStorageKey, CryptoHash, Gas, PanicOnDefault,
    Promise, PromiseResult,
};

const GAS_FOR_ROYALTIES: Gas = 120_000_000_000_000;
const NO_DEPOSIT: Balance = 0;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct ContratoMercado {
    tokens_en_venta: UnorderedMap<TokenKey, TokenEnVenta>,
    tokens_por_id: LookupMap<AccountId, UnorderedSet<TokenId>>,
    tokens_por_id_gate: LookupMap<GateId, UnorderedSet<TokenKey>>,
    tokens_por_id_owner: LookupMap<AccountId, UnorderedSet<TokenKey>>,
    tokens_por_id_creador: LookupMap<AccountId, UnorderedSet<TokenKey>>,
}

/// Cada token debe estar identificado por `<nft contract id, token id>`.
#[derive(BorshDeserialize, BorshSerialize, Serialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct TokenKey(AccountId, TokenId);

impl Display for TokenKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{:?}", self.0, self.1)
    }
}

/// Estructura que representa los tickets en venta
#[derive(BorshDeserialize, BorshSerialize, Serialize)]
#[cfg_attr(not(target_arch = "wasm"), derive(Debug, Deserialize))]
#[serde(crate = "near_sdk::serde")]
pub struct TokenEnVenta {
    pub contrato_id: AccountId,
    pub token_id: TokenId,
    pub owner_id: AccountId,
    pub aprobados_id: U64,
    pub min_precio: U128,
    pub gate_id: Option<GateId>,
    pub id_creador: Option<AccountId>,
}

#[derive(BorshSerialize, BorshStorageKey)]
enum Keys {
    TokensEnVenta,
    TokensPorID,
    TokensPorIdValor(CryptoHash),
    TokensPorIdGate,
    TokensPorIdGateValor(CryptoHash),
    TokensPorIdOwner,
    TokensPorIdOwnerValor(CryptoHash),
    TokensPorIdCreador,
    TokensPorIdCreadorValor(CryptoHash),
}

/// Métodos del contrato market
#[near_log(skip_args, only_pub)]
// #[near_bindgen]
impl ContratoMercado {
    /// Inicializa el contrato
    // #[init]
    pub fn init() -> Self {
        Self {
            tokens_en_venta: UnorderedMap::new(Keys::TokensEnVenta),
            tokens_por_id: LookupMap::new(Keys::TokensPorID),
            tokens_por_id_gate: LookupMap::new(Keys::TokensPorIdGate),
            tokens_por_id_owner: LookupMap::new(Keys::TokensPorIdOwner),
            tokens_por_id_creador: LookupMap::new(Keys::TokensPorIdCreador),
        }
    }

    /// Retorna todos los tokens en venta
    pub fn get_tokens_en_venta(&self) -> Vec<TokenEnVenta> {
        let mut result = Vec::new();
        for (_, token) in self.tokens_en_venta.iter() {
            result.push(token);
        }
        result
    }

    /// Retorna todos los tokens en venta para un `owner_id`
    pub fn get_tokens_by_owner_id(&self, owner_id: ValidAccountId) -> Vec<TokenEnVenta> {
        get_tokens_by(&self.tokens_en_venta, &self.tokens_por_id_owner, owner_id.as_ref())
    }

    /// Retorna todos los tokens en venta para un `id_creador`
    pub fn get_tokens_by_creator_id(&self, id_creador: ValidAccountId) -> Vec<TokenEnVenta> {
        get_tokens_by(&self.tokens_en_venta, &self.tokens_por_id_creador, id_creador.as_ref())
    }

    // #[payable]
    pub fn buy_token(&mut self, contrato_id: ValidAccountId, token_id: TokenId) {
        let token_key = TokenKey(contrato_id.to_string(), token_id);
        if let Some(TokenEnVenta { owner_id, min_precio, gate_id, id_creador, .. }) =
            self.tokens_en_venta.get(&token_key)
        {
            let buyer_id = env::predecessor_account_id();
            if buyer_id == owner_id {
                Panics::BuyOwnTokenNotAllowed.panic();
            }
            let deposit = env::attached_deposit();
            if deposit < min_precio.0 {
                Panics::NotEnoughDepositToBuyToken.panic();
            }
            self.remove_token_por_id(&token_key, &owner_id, &gate_id, &id_creador);
            nep171::nft::nft_transfer_payout(
                buyer_id.try_into().unwrap(),
                token_id,
                None,
                None,
                Some(U128(deposit)),
                &contrato_id,
                0,
                env::prepaid_gas() / 3,
            )
            .then(self_callback::pago(
                &env::current_account_id(),
                NO_DEPOSIT,
                GAS_FOR_ROYALTIES,
            ));
        } else {
            Panics::TokenKeyNotFound { token_key }.panic();
        }
    }

    fn remove_token_por_id(
        &mut self,
        token_key: &TokenKey,
        owner_id: &AccountId,
        gate_id: &Option<GateId>,
        id_creador: &Option<AccountId>,
    ) {
        self.tokens_en_venta.remove(&token_key);
        remove_token_por_id_int(&mut self.tokens_por_id, &token_key, &token_key.0, &token_key.1);
        remove_token_por_id_int(&mut self.tokens_por_id_owner, &token_key, &owner_id, token_key);
        if let Some(gate_id) = gate_id {
            remove_token_por_id_int(&mut self.tokens_por_id_gate, &token_key, &gate_id, token_key);
        }
        if let Some(id_creador) = id_creador {
            remove_token_por_id_int(
                &mut self.tokens_por_id_creador,
                &token_key,
                &id_creador,
                token_key,
            );
        }
    }
}

#[near_ext]
#[ext_contract(self_callback)]
trait SelfCallback {
    fn pago(&mut self);
}

#[near_log(skip_args, only_pub)]
#[near_bindgen]
impl SelfCallback for ContratoMercado {
    #[private]
    fn pago(&mut self) {
        match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Failed => unreachable!(),
            PromiseResult::Successful(value) => {
                if let Ok(payout) = serde_json::from_slice::<Payout>(&value) {
                    for (receiver_id, amount) in payout {
                        Promise::new(receiver_id).transfer(amount.0);
                    }
                } else {
                    unreachable!();
                }
            }
        }
    }
}

/// Implementacion extraida del NEP 171
#[near_log(skip_args, only_pub)]
#[near_bindgen]
impl NonFungibleTokenApprovalsReceiver for ContratoMercado {
    fn nft_on_approve(
        &mut self,
        token_id: TokenId,
        owner_id: ValidAccountId,
        aprobados_id: U64,
        msg: String,
    ) {
        match serde_json::from_str::<MarketApproveMsg>(&msg) {
            Ok(approve_msg) => {
                let contrato_id = env::predecessor_account_id();
                let owner_id = owner_id.to_string();
                self.add_token(&owner_id, &contrato_id, token_id, approve_msg, aprobados_id);
            }
            Err(err) => {
                let reason = err.to_string();
                Panics::MsgFormatMinPriceMissing { reason }.panic();
            }
        }
    }

    fn nft_on_revoke(&mut self, token_id: TokenId) {
        let contrato_id = env::predecessor_account_id();
        let token_key = TokenKey(contrato_id, token_id);

        if let Some(token) = self.tokens_en_venta.get(&token_key) {
            assert_eq!(token.contrato_id, token_key.0);
            self.remove_token_por_id(&token_key, &token.owner_id, &token.gate_id, &token.id_creador);
        } else {
            Panics::TokenKeyNotFound { token_key }.panic();
        }
    }

    fn batch_on_approve(
        &mut self,
        tokens: Vec<(TokenId, MarketApproveMsg)>,
        owner_id: ValidAccountId,
    ) {
        let contrato_id = env::predecessor_account_id();
        let owner_id = owner_id.to_string();
        for (token_id, approve_msg) in tokens {
            self.add_token(&owner_id, &contrato_id, token_id, approve_msg, U64(0));
        }
    }
}

/// Implementacion del contrato
impl ContratoMercado {
    fn add_token(
        &mut self,
        owner_id: &AccountId,
        contrato_id: &String,
        token_id: TokenId,
        approve_msg: MarketApproveMsg,
        aprobados_id: U64,
    ) {
        let token_key = TokenKey(contrato_id.clone(), token_id);
        self.tokens_en_venta.insert(
            &token_key,
            &TokenEnVenta {
                contrato_id: contrato_id.clone(),
                token_id,
                owner_id: owner_id.clone().into(),
                aprobados_id,
                min_precio: approve_msg.min_precio,
                gate_id: approve_msg.gate_id.clone().map(|g| g.to_string()),
                id_creador: approve_msg.id_creador.clone(),
            },
        );

        insert_token_id_to(
            &mut self.tokens_por_id,
            &contrato_id,
            &token_id,
            Keys::TokensPorIdValor,
        );
        insert_token_id_to(
            &mut self.tokens_por_id_owner,
            &owner_id.into(),
            &token_key,
            Keys::TokensPorIdOwnerValor,
        );
        if let Some(gate_id) = approve_msg.gate_id {
            insert_token_id_to(
                &mut self.tokens_por_id_gate,
                gate_id.as_ref(),
                &token_key,
                Keys::TokensPorIdGateValor,
            );
        }
        if let Some(id_creador) = approve_msg.id_creador {
            insert_token_id_to(
                &mut self.tokens_por_id_creador,
                &id_creador,
                &token_key,
                Keys::TokensPorIdCreadorValor,
            );
        }
    }
}

fn insert_token_id_to<T: BorshSerialize + BorshDeserialize, F: FnOnce(CryptoHash) -> Keys>(
    tokens_map: &mut LookupMap<String, UnorderedSet<T>>,
    key: &String,
    token_key: &T,
    f: F,
) {
    let mut tids = tokens_map.get(&key).unwrap_or_else(|| UnorderedSet::new(f(crypto_hash(key))));
    tids.insert(token_key);
    tokens_map.insert(key, &tids);
}

fn get_tokens_by<K: BorshSerialize>(
    ts: &UnorderedMap<TokenKey, TokenEnVenta>,
    tokens_map: &LookupMap<K, UnorderedSet<TokenKey>>,
    key: &K,
) -> Vec<TokenEnVenta> {
    match tokens_map.get(&key) {
        None => Vec::new(),
        Some(tids) => {
            tids.iter().map(|token_id| ts.get(&token_id).expect("Token not found")).collect()
        }
    }
}

fn remove_token_por_id_int<T: BorshSerialize + BorshDeserialize + Clone, K: BorshSerialize>(
    tokens_map: &mut LookupMap<K, UnorderedSet<T>>,
    t: &TokenKey,
    key: &K,
    token_key: &T,
) {
    match tokens_map.get(&key) {
        None => Panics::TokenKeyNotFound { token_key: t.clone() }.panic(),
        Some(mut tids) => {
            if !tids.remove(token_key) {
                Panics::TokenKeyNotFound { token_key: t.clone() }.panic();
            }

            tokens_map.insert(&key, &tids);
        }
    }
}

/// Posibles errores de tipo Panic
#[derive(Serialize, PanicMessage)]
#[serde(crate = "near_sdk::serde", tag = "err")]
pub enum Panics {
    /// Thrown when `nft_on_approve` does not find `min_precio`.
    #[panic_msg = "Could not find min_precio in msg: {}"]
    MsgFormatMinPriceMissing { reason: String },
    /// Thrown when the `token_key` was not found.
    #[panic_msg = "Token Key `{}` was not found"]
    TokenKeyNotFound { token_key: TokenKey },
    /// Thrown when buyer attempts to buy own token.
    #[panic_msg = "Buyer cannot buy own token"]
    BuyOwnTokenNotAllowed,
    /// Thrown when deposit is not enough to buy a token.
    #[panic_msg = "Not enough deposit to cover token minimum price"]
    NotEnoughDepositToBuyToken,
}