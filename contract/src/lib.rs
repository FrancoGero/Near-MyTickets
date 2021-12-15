/// NEAR DAPP, My Ticket
/// @Dev Franco Geroli
/// 
/// - Las funciones relacionadas a un token retornan su metadata para su 
/// posterior uso en una API

/// Importación de librerías y módulos
pub use crate::core_nft::*;
pub use crate::market::*;
pub use crate::context::*;
mod core_nft;
mod market;
mod context;

use fraction::Fraction;
use nep171::NonFungibleTokenCore;
use nep177::{NFTContractMetadata, NonFungibleTokenMetadata};
use nep178::NonFungibleTokenApprovalMgmt;
use nep181::NonFungibleTokenEnumeration;
use near_env::{near_ext, near_log, PanicMessage};
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    collections::{LookupMap, UnorderedMap, UnorderedSet},
    env, ext_contract,
    json_types::{ValidAccountId, U128, U64},
    log, near_bindgen,
    serde::{Deserialize, Serialize},
    serde_json, setup_alloc, AccountId, Balance, BorshStorageKey, CryptoHash, Gas, PanicOnDefault,
    Promise, PromiseResult,
};
use std::{collections::HashMap, convert::TryInto, fmt::Display};

setup_alloc!();

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct ContratoNft {
    /// La coleccion de tickets se guardan en mappings
    tickets: UnorderedMap<GateId, Collectible>,
    tickets_de_creador: LookupMap<AccountId, UnorderedSet<GateId>>,
    tokens: UnorderedMap<TokenId, Token>,
    tokens_de_address: LookupMap<AccountId, UnorderedSet<TokenId>>,

    id_admin: AccountId,
    metadata: NFTContractMetadata,
    fee_reventa: Fraction,
    fee_reventa_id_address: AccountId,
}

#[derive(BorshSerialize, BorshStorageKey)]
enum Keys {
    Tickets,
    TicketsPorCreador,
    TicketsPorCreadorValor { hash_id_creador: CryptoHash },
    Tokens,
    TokensPorDueño,
    TokensPorDueñoValor { hash_id_dueño: CryptoHash },
}

/// Metodos del contrato principal
#[near_log(skip_args, only_pub)]
#[near_bindgen]
impl ContratoNft {
    /// Inicializa el contrato, si no se le llama explicitamente retorna un Panic
    ///
    /// `id_admin` es la cuenta del contrato
    /// `min_comision` y `max_comision` indicates what must be the max and min comision respectively when creating a ticket.
    /// `fee_reventa` es el porcentaje que se paga a `fee_reventa_id_address` al revender
    #[init]
    pub fn init(
        id_admin: ValidAccountId,
        metadata: NFTContractMetadata,
        min_comision: Fraction,
        max_comision: Fraction,
        fee_reventa: Fraction,
        fee_reventa_id_address: ValidAccountId,
    ) -> Self {
        min_comision.check();
        max_comision.check();
        fee_reventa.check();

        Self {
            tickets: UnorderedMap::new(Keys::Tickets),
            tickets_de_creador: LookupMap::new(Keys::TicketsPorCreador),
            tokens: UnorderedMap::new(Keys::Tokens),
            tokens_de_address: LookupMap::new(Keys::TokensPorDueño),
            id_admin: id_admin.as_ref().to_string(),
            metadata,
            fee_reventa,
            fee_reventa_id_address: fee_reventa_id_address.to_string(),
        }
    }

    /// Crea una nueva seri de tickets indentificando por IDs
    /// `cantidad` indica la cantidad maxima
    /// `comision` indica la comision, en porcentaje, que se paga al creador al momento de la venta
    ///
    /// Entre comision y fee no pueden superar 1, de lo contrario da error
    pub fn crear_ticket(
        &mut self,
        id_creador: ValidAccountId,
        gate_id: ValidGateId,
        titulo: String,
        descripcion: String,
        cantidad: u16,
        comision: Fraction,
        media: Option<String>,
        media_hash: Option<String>,
        referencia: Option<String>,
        referencia_hash: Option<String>,
    ) {
        let gate_id = gate_id.to_string();

        let bn = 1_000_000_000_000_000_000_000;
        if self.fee_reventa.mult(bn) + comision.mult(bn) >= bn {
            Panic::RoyaltyTooLarge { comision, fee_reventa: self.fee_reventa }.panic();
        }
        if self.tickets.get(&gate_id).is_some() {
            Panic::GateIdAlreadyExists { gate_id }.panic();
        }
        if cantidad == 0 {
            Panic::ZeroSupplyNotAllowed { gate_id }.panic();
        }
        if titulo.len() > 140 {
            Panic::InvalidArgument { gate_id, reason: "Titulo no puede tener mas de 140 caracteres".to_string() }
                .panic();
        }
        if descripcion.len() > 1024 {
            Panic::InvalidArgument {
                gate_id,
                reason: "`La descripcion no puede sobrepasar los 1024 caracteres".to_string(),
            }
            .panic();
        }

        macro_rules! check {
            ($arg:ident) => {{
                if let Some(val) = &$arg {
                    if val.len() > 1024 {
                        Panic::InvalidArgument {
                            gate_id,
                            reason: concat!("`", stringify!($arg), "` exceeds 1024 chars")
                                .to_string(),
                        }
                        .panic();
                    }
                }
            }};
        }

        check!(media);
        check!(media_hash);
        check!(referencia);
        check!(referencia_hash);

        if env::predecessor_account_id() != self.id_admin {
            Panic::AdminRestrictedOperation.panic();
        }

        let id_creador = AccountId::from(id_creador);
        let ahora = env::block_timestamp() / 1_000_000;

        let ticket = Collectible {
            gate_id,
            id_creador,
            cantidad_actual: cantidad,
            tokens_creados: Vec::new(),
            comision,
            metadata: Metadata {
                titulo: Some(titulo),
                descripcion: Some(descripcion),
                media,
                media_hash,
                copias: Some(cantidad),
                emitido_en: Some(ahora),
                expira_en: None,
                comienzo_en: Some(ahora),
                actualizado_en: None,
                extra: None,
                referencia,
                referencia_hash,
            },
        };
        self.tickets.insert(&ticket.gate_id, &ticket);

        let mut guia =
            self.tickets_de_creador.get(&ticket.id_creador).unwrap_or_else(|| {
                UnorderedSet::new(Keys::TicketsPorCreadorValor {
                    hash_id_creador: crypto_hash(&ticket.id_creador),
                })
            });
        guia.insert(&ticket.gate_id);

        self.tickets_de_creador.insert(&ticket.id_creador, &guia);
    }

    /// Retona un ticket indicado segun ID
    pub fn get_ticket_por_id(&self, gate_id: ValidGateId) -> Option<Collectible> {
        let gate_id = gate_id.to_string();

        match self.tickets.get(&gate_id) {
            None => None,
            Some(ticket) => {
                assert!(ticket.gate_id == gate_id);
                Some(ticket)
            }
        }
    }

    /// Retorna los ticket de un creador
    pub fn get_tickets_de_creador(&self, id_creador: ValidAccountId) -> Vec<Collectible> {
        match self.tickets_de_creador.get(id_creador.as_ref()) {
            None => Vec::new(),
            Some(list) => list
                .iter()
                .map(|gate_id| {
                    let ticket = self.tickets.get(&gate_id).expect("Gate Id not found");
                    assert!(ticket.gate_id == gate_id);
                    assert!(&ticket.id_creador == id_creador.as_ref());
                    ticket
                })
                .collect(),
        }
    }

    /// Elimina ticket segun ID indicado
    /// Puede ejecutarse solo por `id_creador` y `id_admin` 
    pub fn borrar_ticket(&mut self, gate_id: ValidGateId) {
        let gate_id: GateId = From::from(gate_id);
        match self.tickets.get(&gate_id) {
            None => Panic::GateIdNotFound { gate_id }.panic(),
            Some(ticket) => {
                assert!(ticket.gate_id == gate_id);
                if !ticket.tokens_creados.is_empty() {
                    Panic::GateIdHasTokens { gate_id }.panic();
                }
                let pred_id = env::predecessor_account_id();
                if pred_id == ticket.id_creador || pred_id == self.id_admin {
                    self.tickets.remove(&gate_id).unwrap();

                    let mut cs = self.tickets_de_creador.get(&ticket.id_creador).unwrap();
                    let removed = cs.remove(&gate_id);
                    assert!(removed);
                    self.tickets_de_creador.insert(&ticket.id_creador, &cs);
                } else {
                    Panic::NotAuthorized { gate_id }.panic();
                }
            }
        }
    }

    /// Permite la comprar de un token y retorna token ID
    pub fn comprar_token(&mut self, gate_id: ValidGateId) -> TokenId {
        let gate_id = gate_id.to_string();

        match self.tickets.get(&gate_id) {
            None => Panic::GateIdNotFound { gate_id }.panic(),
            Some(mut ticket) => {
                if ticket.cantidad_actual == 0 {
                    Panic::GateIdExhausted { gate_id }.panic()
                }

                let owner_id = env::predecessor_account_id();
                let ahora = env::block_timestamp() / 1_000_000;

                let token_id = self.tokens.len();
                let token = Token {
                    token_id: U64::from(token_id),
                    gate_id: gate_id.clone(),
                    owner_id,
                    created_at: ahora,
                    modified_at: ahora,
                    approvals: HashMap::new(),
                    approval_counter: U64::from(0),
                    metadata: Metadata::default(),
                };
                self.insertar_token(&token);

                ticket.cantidad_actual = ticket.cantidad_actual - 1;
                ticket.tokens_creados.push(U64(token_id));
                self.tickets.insert(&gate_id, &ticket);

                U64::from(token_id)
            }
        }
    }


    /* 
     *   Funciones internas 
     */

    /// Retorna todos los tokens de un dueño indicado por ID
    pub fn get_tokens_de_dueno(&self, owner_id: ValidAccountId) -> Vec<Token> {
        match self.tokens_de_address.get(owner_id.as_ref()) {
            None => Vec::new(),
            Some(list) => list
                .iter()
                .map(|token_id| {
                    let token = self.get_token(token_id).expect("No se encuentra el token indicado");
                    assert!(token.token_id == token_id);
                    assert!(&token.owner_id == owner_id.as_ref());
                    token
                })
                .collect(),
        }
    }

    /// Retorna un token según `token_id`, o un None si no existe
    fn get_token(&self, token_id: TokenId) -> Option<Token> {
        match self.tokens.get(&token_id) {
            None => None,
            Some(mut token) => {
                assert!(token.token_id == token_id);
                let ticket = self.tickets.get(&token.gate_id).expect("ID no encontrado");
                token.metadata = ticket.metadata;
                Some(token)
            }
        }
    }

    /// Funcion interna que retorna el token segun ID o da un Panic error
    fn get_token_int(&self, token_id: TokenId) -> Token {
        match self.get_token(token_id) {
            None => Panic::TokenIdNotFound { token_id }.panic(),
            Some(token) => token,
        }
    }

    /// Añade un token en `tokens` y en `tokens_de_address`.
    fn insertar_token(&mut self, token: &Token) {
        self.tokens.insert(&token.token_id, token);

        let mut tids = self.tokens_de_address.get(&token.owner_id).unwrap_or_else(|| {
            UnorderedSet::new(Keys::TokensPorDueñoValor {
                hash_id_dueño: crypto_hash(&token.owner_id),
            })
        });
        tids.insert(&token.token_id);

        self.tokens_de_address.insert(&token.owner_id, &tids);
    }

    /// Metodo interno llamado por borrar token
    fn borrar_token_int(&mut self, token_id: TokenId, owner_id: &AccountId) {
        match self.tokens_de_address.get(&owner_id) {
            None => Panic::TokenIdNotOwnedBy { token_id, owner_id: owner_id.clone() }.panic(),
            Some(mut list) => {
                if !list.remove(&token_id) {
                    Panic::TokenIdNotOwnedBy { token_id, owner_id: owner_id.clone() }.panic();
                }
                self.tokens_de_address.insert(&owner_id, &list);

                let was_removed = self.tokens.remove(&token_id);
                assert!(was_removed.is_some());
            }
        }
    }

    /// Aprobar un token por lote
    pub fn aprobar_por_lote(
        &mut self,
        tokens: Vec<(TokenId, U128)>,
        account_id: ValidAccountId,
    ) -> Promise {
        if tokens.len() > 10 {
            Panic::ExceedTokensToBatchApprove.panic();
        }

        let owner_id = env::predecessor_account_id();
        let mut oks = Vec::new();
        let mut errs = Vec::new();
        for (token_id, min_precio) in tokens {
            match self.aprobar_token(token_id, &owner_id, account_id.to_string(), min_precio) {
                Ok(msg) => oks.push((token_id, msg)),
                Err(err) => errs.push((token_id, err)),
            }
        }
        core_nft::nep178::market::batch_on_approve(
            oks,
            owner_id.try_into().unwrap(),
            account_id.as_ref(),
            NO_DEPOSIT,
            GAS_FOR_ROYALTIES,
        )
        .then(self_callback::resolve_batch_approve(
            errs,
            &env::current_account_id(),
            NO_DEPOSIT,
            GAS_FOR_ROYALTIES,
        ))
    }

    fn aprobar_token(
        &mut self,
        token_id: TokenId,
        owner_id: &AccountId,
        account_id: AccountId,
        min_precio: U128,
    ) -> Result<MarketApproveMsg, Panic> {
        let mut token = match self.tokens.get(&token_id) {
            None => return Err(Panic::TokenIdNotFound { token_id }),
            Some(token) => token,
        };
        if owner_id != &token.owner_id {
            return Err(Panic::TokenIdNotOwnedBy { token_id, owner_id: owner_id.clone() });
        }
        if token.approvals.len() > 0 {
            return Err(Panic::OneApprovalAllowed);
        }

        token.approval_counter.0 = token.approval_counter.0 + 1;
        token
            .approvals
            .insert(account_id, TokenApproval { aprobados_id: token.approval_counter, min_precio });
        self.tokens.insert(&token_id, &token);
        match self.tickets.get(&token.gate_id) {
            None => Err(Panic::GateIdNotFound { gate_id: token.gate_id }),
            Some(ticket) => Ok(MarketApproveMsg {
                min_precio,
                gate_id: Some(token.gate_id.try_into().unwrap()),
                id_creador: Some(ticket.id_creador),
            }),
        }
    }
}

/// Implementacion Token no fungible según NEP 171
#[near_log(skip_args, only_pub)]
#[near_bindgen]
impl NonFungibleTokenCore for ContratoNft {
    fn nft_transfer(
        &mut self,
        receiver_id: ValidAccountId,
        token_id: TokenId,
        enforce_approval_id: Option<U64>,
        memo: Option<String>,
    ) {
        let sender_id = env::predecessor_account_id();
        let mut token = self.get_token_int(token_id);

        if sender_id != token.owner_id && token.approvals.get(&sender_id).is_none() {
            Panic::SenderNotAuthToTransfer { sender_id }.panic();
        }

        if &token.owner_id == receiver_id.as_ref() {
            Panic::ReceiverIsOwner.panic();
        }

        if let Some(enforce_approval_id) = enforce_approval_id {
            let TokenApproval { aprobados_id, min_precio: _ } = token
                .approvals
                .get(receiver_id.as_ref())
                .expect("Receiver not an approver of this token.");
            if aprobados_id != &enforce_approval_id {
                Panic::EnforceApprovalFailed.panic();
            }
        }
        if let Some(memo) = memo {
            log!("Memo: {}", memo);
        }

        self.borrar_token_int(token_id, &token.owner_id);

        token.owner_id = receiver_id.as_ref().to_string();
        token.modified_at = env::block_timestamp() / 1_000_000;
        token.approvals.clear();
        self.insertar_token(&token);
    }

    fn nft_payout(&self, token_id: TokenId, balance: U128) -> Payout {
        let token = self.get_token_int(token_id);
        match self.tickets.get(&token.gate_id) {
            None => Panic::GateIdNotFound { gate_id: token.gate_id }.panic(),
            Some(ticket) => {
                let royalty_amount = ticket.comision.mult(balance.0);
                let fee_amount = self.fee_reventa.mult(balance.0);
                let owner_amount = balance.0 - royalty_amount - fee_amount;
                let entries = vec![
                    (ticket.id_creador, royalty_amount),
                    (self.fee_reventa_id_address.clone(), fee_amount),
                    (token.owner_id, owner_amount),
                ];

                let mut payout = HashMap::new();
                for (account_id, amount) in entries {
                    payout.entry(account_id).or_insert(U128(0)).0 += amount;
                }
                payout
            }
        }
    }

    fn nft_transfer_payout(
        &mut self,
        receiver_id: ValidAccountId,
        token_id: TokenId,
        aprobados_id: Option<U64>,
        memo: Option<String>,
        balance: Option<U128>,
    ) -> Option<Payout> {
        let payout = balance.map(|balance| self.nft_payout(token_id, balance));
        self.nft_transfer(receiver_id, token_id, aprobados_id, memo);
        payout
    }

    fn nft_token(&self, token_id: TokenId) -> Option<Token> {
        self.get_token(token_id)
    }
}

/// Implementacion Token no fungible según NEP 177
#[near_log(skip_args, only_pub)]
#[near_bindgen]
impl NonFungibleTokenMetadata for ContratoNft {
    fn nft_metadata(&self) -> NFTContractMetadata {
        self.metadata.clone()
    }
}

/// Implementacion Token no fungible según NEP 178
#[near_log(skip_args, only_pub)]
#[near_bindgen]
impl NonFungibleTokenApprovalMgmt for ContratoNft {
    fn nft_approve(
        &mut self,
        token_id: TokenId,
        account_id: ValidAccountId,
        msg: Option<String>,
    ) -> Promise {
        let min_precio = {
            if let Some(msg) = msg.clone() {
                match serde_json::from_str::<NftApproveMsg>(&msg) {
                    Ok(approve_msg) => approve_msg.min_precio,
                    Err(err) => Panic::MsgFormatMinPriceMissing { reason: err.to_string() }.panic(),
                }
            } else {
                Panic::MsgFormatNotRecognized.panic();
            }
        };

        let owner_id = env::predecessor_account_id();
        let mut token = self.get_token_int(token_id);
        if &owner_id != &token.owner_id {
            Panic::TokenIdNotOwnedBy { token_id, owner_id }.panic();
        }
        if token.approvals.len() > 0 {
            Panic::OneApprovalAllowed.panic();
        }

        token.approval_counter.0 = token.approval_counter.0 + 1;
        token.approvals.insert(
            account_id.clone().into(),
            TokenApproval { aprobados_id: token.approval_counter, min_precio },
        );
        self.tokens.insert(&token_id, &token);

        match self.tickets.get(&token.gate_id) {
            None => Panic::GateIdNotFound { gate_id: token.gate_id }.panic(),
            Some(ticket) => {
                let market_msg = MarketApproveMsg {
                    min_precio,
                    gate_id: Some(token.gate_id.try_into().unwrap()),
                    id_creador: Some(ticket.id_creador),
                };
                core_nft::nep178::market::nft_on_approve(
                    token_id,
                    owner_id.try_into().unwrap(),
                    U64::from(token.approval_counter),
                    serde_json::to_string(&market_msg).unwrap(),
                    account_id.as_ref(),
                    0,
                    env::prepaid_gas() / 2,
                )
            }
        }
    }

    fn nft_revoke(&mut self, token_id: TokenId, account_id: ValidAccountId) -> Promise {
        let owner_id = env::predecessor_account_id();
        let mut token = self.get_token_int(token_id);
        if &owner_id != &token.owner_id {
            Panic::TokenIdNotOwnedBy { token_id, owner_id }.panic();
        }
        if token.approvals.remove(account_id.as_ref()).is_none() {
            Panic::RevokeApprovalFailed { account_id: account_id.to_string() }.panic();
        }
        self.tokens.insert(&token_id, &token);
        core_nft::nep178::market::nft_on_revoke(
            token_id,
            account_id.as_ref(),
            0,
            env::prepaid_gas() / 2,
        )
    }

    fn nft_revoke_all(&mut self, token_id: TokenId) {
        let owner_id = env::predecessor_account_id();
        let mut token = self.get_token_int(token_id);
        if &owner_id != &token.owner_id {
            Panic::TokenIdNotOwnedBy { token_id, owner_id }.panic();
        }
        for (nft_id, _) in &token.approvals {
            core_nft::nep178::market::nft_on_revoke(token_id, nft_id, 0, env::prepaid_gas() / 2);
        }
        token.approvals.clear();
        self.tokens.insert(&token_id, &token);
    }
}

/// Implementacion Token no fungible según NEP 181
#[near_log(skip_args, only_pub)]
#[near_bindgen]
impl NonFungibleTokenEnumeration for ContratoNft {
    /// Returns the total token cantidad.
    fn nft_total_supply(&self) -> U64 {
        U64::from(self.tokens.len())
    }

    fn nft_tokens(&self, from_index: Option<U64>, limit: Option<u32>) -> Vec<Token> {
        let mut i = from_index.map_or(0, |s| s.0);
        let mut result = Vec::new();
        while result.len() < limit.unwrap_or(u32::MAX) as usize {
            if let Some(mut token) = self.tokens.values_as_vector().get(i) {
                let ticket = self.tickets.get(&token.gate_id).expect("Gate id not found");
                token.metadata = ticket.metadata;
                result.push(token);
                i += 1
            } else {
                break;
            }
        }
        result
    }

    fn nft_supply_for_owner(&self, account_id: ValidAccountId) -> U64 {
        match self.tokens_de_address.get(account_id.as_ref()) {
            None => 0.into(),
            Some(list) => list.len().into(),
        }
    }

    fn nft_tokens_for_owner(
        &self,
        account_id: ValidAccountId,
        from_index: Option<U64>,
        limit: Option<u32>,
    ) -> Vec<Token> {
        match self.tokens_de_address.get(account_id.as_ref()) {
            None => Vec::new(),
            Some(list) => {
                let mut i = from_index.map_or(0, |s| s.0);
                let mut result = Vec::new();
                while result.len() < limit.unwrap_or(u32::MAX) as usize {
                    if let Some(token_id) = list.as_vector().get(i) {
                        let token = self.get_token(token_id).expect("Token not found");
                        assert!(token.token_id == token_id);
                        assert!(&token.owner_id == account_id.as_ref());
                        result.push(token);
                        i += 1
                    } else {
                        break;
                    }
                }
                result
            }
        }
    }

    fn nft_token_uri(&self, token_id: TokenId) -> Option<String> {
        self.metadata.base_uri.clone().and_then(|uri| {
            self.tokens.get(&token_id).map(|t| {
                let sep = if uri.ends_with("/") { "" } else { "/" };
                format!("{}{}{}", uri, sep, t.gate_id)
            })
        })
    }
}

const GAS_FOR_ROYALTIES: Gas = 120_000_000_000_000;
const NO_DEPOSIT: Balance = 0;

#[near_ext]
#[ext_contract(self_callback)]
trait SelfCallback {
    fn resolve_batch_approve(&mut self, errs: Vec<(TokenId, Panic)>);
}

#[near_log(skip_args, only_pub)]
#[near_bindgen]
impl SelfCallback for ContratoNft {
    #[private]
    fn resolve_batch_approve(&mut self, errs: Vec<(TokenId, Panic)>) {
        match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Failed => unreachable!(),
            PromiseResult::Successful(_) => {
                if !errs.is_empty() {
                    Panic::Errors { panics: Panics(errs) }.panic()
                }
            }
        }
    }
}

/// Posibles errores que se usan posteriormente como Panic error
#[derive(Serialize, Deserialize, PanicMessage)]
#[serde(crate = "near_sdk::serde", tag = "err")]
pub enum Panic {
    #[panic_msg = "Royalty `{}` of `{}` is less than min"]
    RoyaltyMinThanAllowed { comision: Fraction, gate_id: String },
    #[panic_msg = "Royalty `{}` of `{}` is greater than max"]
    RoyaltyMaxThanAllowed { comision: Fraction, gate_id: String },
    #[panic_msg = "Royalty `{}` is too large for the given NFT fee `{}`"]
    RoyaltyTooLarge { comision: Fraction, fee_reventa: Fraction },
    #[panic_msg = "Gate ID `{}` already exists"]
    GateIdAlreadyExists { gate_id: GateId },
    #[panic_msg = "Gate ID `{}` must have a positive cantidad"]
    ZeroSupplyNotAllowed { gate_id: GateId },
    #[panic_msg = "Invalid argument for gate ID `{}`: {}"]
    InvalidArgument { gate_id: GateId, reason: String },
    #[panic_msg = "Operation is allowed only for admin"]
    AdminRestrictedOperation,
    #[panic_msg = "Gate ID `{}` was not found"]
    GateIdNotFound { gate_id: GateId },
    #[panic_msg = "Tokens for gate id `{}` have already been claimed"]
    GateIdExhausted { gate_id: GateId },
    #[panic_msg = "Gate ID `{}` has already some claimed tokens"]
    GateIdHasTokens { gate_id: GateId },
    #[panic_msg = "Unable to delete gate ID `{}`"]
    NotAuthorized { gate_id: GateId },
    #[panic_msg = "Token ID `{:?}` was not found"]
    TokenIdNotFound { token_id: U64 },
    #[panic_msg = "Token ID `{:?}` does not belong to account `{}`"]
    TokenIdNotOwnedBy { token_id: U64, owner_id: AccountId },
    #[panic_msg = "At most one approval is allowed per Token"]
    OneApprovalAllowed,
    #[panic_msg = "Sender `{}` is not authorized to make transfer"]
    SenderNotAuthToTransfer { sender_id: AccountId },
    #[panic_msg = "The token owner and the receiver should be different"]
    ReceiverIsOwner,
    #[panic_msg = "The aprobados_id is different from enforce_approval_id"]
    EnforceApprovalFailed,
    #[panic_msg = "The msg argument must contain the minimum price"]
    MsgFormatNotRecognized,
    #[panic_msg = "Could not find min_precio in msg: {}"]
    MsgFormatMinPriceMissing { reason: String },
    #[panic_msg = "Could not revoke approval for `{}`"]
    RevokeApprovalFailed { account_id: AccountId },
    #[panic_msg = "At most 10 tokens are allowed to approve in batch"]
    ExceedTokensToBatchApprove,
    #[panic_msg = "{} error(s) detected, see `panics` fields for a full list of errors"]
    Errors { panics: Panics },
}

/// Almacena los errores identificando por `TokenId`
#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Panics(pub Vec<(TokenId, Panic)>);

impl Display for Panics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.len())
    }
}

