use core_nft::{
    gate::{GateId, ValidGateId},
    mock_context,
    mocked_context::{
        alice, any, bob, charlie, gate_id, market, mintgate_admin, mintgate_fee_account_id,
    },
    nep171::NonFungibleTokenCore,
    nep177::NFTContractMetadata,
    nep177::NonFungibleTokenMetadata,
    nep178::NonFungibleTokenApprovalMgmt,
    nep181::NonFungibleTokenEnumeration,
    NftApproveMsg, TokenApproval, TokenId,
};
use mg_nft::NftContract;
use near_sdk::{
    json_types::{ValidAccountId, U128, U64},
    serde_json,
};
use std::{
    convert::TryInto,
    ops::{Deref, DerefMut},
};

// mock_context!();

struct NftContractChecker {
    contrato: NftContract,
    claimed_tokens: Vec<TokenId>,
}

impl MockedContext<NftContractChecker> {

    fn inicializar_contrato(
        min_royalty: &str,
        max_royalty: &str,
        metadata: NFTContractMetadata,
    ) -> MockedContext<NftContractChecker> {
        MockedContext::new(|| NftContractChecker {
            contrato: NftContract::init(
                mintgate_admin(),
                metadata,
                min_royalty.parse().unwrap(),
                max_royalty.parse().unwrap(),
                "25/1000".parse().unwrap(),
                mintgate_fee_account_id(),
            ),
            claimed_tokens: Vec::new(),
        })
    }
    
    fn init() -> MockedContext<NftContractChecker> {
        init_contract("5/100", "30/100", metadata(base_uri()))
    }
    
    fn metadata(base_uri: Option<String>) -> NFTContractMetadata {
        NFTContractMetadata {
            spec: "my-tickets".to_string(),
            name: "Tickets App".to_string(),
            symbol: "MTK".to_string(),
            icon: None,
            base_uri,
            reference: None,
            reference_hash: None,
        }
    }
    fn crear_tickets(
        &mut self,
        creador_id: ValidAccountId,
        gate_id: ValidGateId,
        cantidad: u16,
        comision: &str,
    ) {
        let tickets_por_owner = self.get_collectibles_by_creator(creator_id.clone());

        println!("Tickets: `{}`, supply {}", gate_id, cantidad);

        let comision = comision.parse().unwrap();
        self.contrato.crear_tickets(
            creador_id.clone(),
            gate_id.clone(),
            "My tickets".to_string(),
            "descripcion".to_string(),
            cantidad,
            comision,
            Some("texto".to_string()),
            Some("111".to_string()),
            Some("ref".to_string()),
            Some("222".to_string()),
        );

        let ticket = self.contrato.get_collectible_by_gate_id(gate_id.clone()).unwrap();
        assert_eq!(ticket.creator_id, creator_id.to_string());
        assert_eq!(&ticket.gate_id, gate_id.as_ref());
        assert_eq!(ticket.current_supply, supply);
        assert_eq!(ticket.minted_tokens.len(), 0);
        assert_eq!(ticket.royalty, royalty);
        assert_eq!(ticket.metadata.media, Some("media".to_string()));
        assert_eq!(ticket.metadata.media_hash, Some("111".to_string()));
        assert_eq!(ticket.metadata.reference, Some("ref".to_string()));
        assert_eq!(ticket.metadata.reference_hash, Some("222".to_string()));

        assert_eq!(
            self.get_collectibles_by_creator(creador_id).len(),
            tickets_por_owner.len() + 1
        );
    }

    fn comprar_tickets(&mut self, gate_id: ValidGateId) -> TokenId {
        let total_supply = self.
        .nft_total_supply().0;
        let supply_por_owner = self.contrato.nft_supply_for_owner(self.pred_id()).0;

        let token_id = self.contrato.claim_token(gate_id.clone());

        assert_eq!(self.contrato.nft_total_supply(), U64(total_supply + 1));
        assert_eq!(self.contrato.nft_supply_for_owner(self.pred_id()), U64(supply_por_owner + 1));

        let token = self.nft_token(token_id).unwrap();
        assert_eq!(&token.gate_id, gate_id.as_ref());
        assert_eq!(token.owner_id, self.pred_id().to_string());
        assert_eq!(token.approvals.len(), 0);
        assert_eq!(token.approval_counter, U64(0));

        let ticket = self.contrato.get_collectible_by_gate_id(gate_id.clone()).unwrap();
        assert_eq!(token.metadata, ticket.metadata);

        self.claimed_tokens.insert(0, token_id);
        token_id
    }

}

