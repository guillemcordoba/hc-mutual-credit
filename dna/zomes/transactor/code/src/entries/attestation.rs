use crate::offer;
use hdk::entry_definition::ValidatingEntryType;
use hdk::holochain_json_api::{error::JsonError, json::JsonString};
use hdk::holochain_persistence_api::cas::content::Address;
use hdk::prelude::AddressableContent;
use hdk::{
    error::{ZomeApiError, ZomeApiResult},
    holochain_core_types::{
        chain_header::ChainHeader, dna::entry_types::Sharing, link::LinkMatch, time::Iso8601,
    },
    ValidationData,
};
use holochain_entry_utils::HolochainEntry;
use holochain_wasm_utils::api_serialization::get_links::LinksResult;

#[derive(Serialize, Deserialize, Debug, DefaultJson, Clone)]
pub struct Attestation {
    pub header_addresses: Vec<Address>,
}

impl Attestation {
    pub fn from_headers(chain_headers: &Vec<ChainHeader>) -> Attestation {
        let header_addresses = chain_headers.iter().map(|h| h.address()).collect();

        Attestation { header_addresses }
    }
}

impl HolochainEntry for Attestation {
    fn entry_type() -> String {
        String::from("attestation")
    }
}

pub fn entry_definition() -> ValidatingEntryType {
    entry!(
        name: Attestation::entry_type(),
        description: "attestation entry to vouch for the last transaction of an agent",
        sharing: Sharing::Public,
        validation_package: || {
            hdk::ValidationPackageDefinition::Entry
        },
        validation: |_validation_data: hdk::EntryValidationData<Attestation>| {
            match _validation_data {
                hdk::EntryValidationData::Create { entry, validation_data } => validate_attestation(entry, validation_data),
            _ => Err(String::from("Delete attestation is not allowed"))
            }
        },
        links:[
            from!(
                "%agent_id",
                link_type: "agent->attestation",
                validation_package: || {
                    hdk::ValidationPackageDefinition::Entry
                },
                validation: | _validation_data: hdk::LinkValidationData | {
                    Ok(())
                }
            )
        ]
    )
}

pub fn validate_attestation(
    attestation: Attestation,
    _validation_data: ValidationData,
) -> Result<(), String> {
    let chain_headers = attestation
        .header_addresses
        .into_iter()
        .map(|header_address| hdk::utils::get_as_type(header_address))
        .collect::<ZomeApiResult<Vec<ChainHeader>>>()?;

    validate_transaction_headers(&chain_headers)?;

    Ok(())
}

/**
 * Gets the last attestation from the DHT for the given agent and the number of attestations present in the DHT
 */
pub fn get_latest_attestation_for(
    agent_address: &Address,
) -> ZomeApiResult<(Option<Attestation>, usize)> {
    let links_result = hdk::get_links(
        &agent_address,
        LinkMatch::Exactly("agent->attestation"),
        LinkMatch::Any,
    )?;

    let links = links_result.links();

    if links.len() == 0 {
        return Ok((None, 0));
    }

    let mut non_agent_links: Vec<LinksResult> = links
        .into_iter()
        .filter(|link| {
            link.headers
                .iter()
                .find(|h| h.provenances()[0].source() != agent_address.clone())
                .is_some()
        })
        .collect();

    let first_timestamp = |link: LinksResult| {
        link.headers
            .into_iter()
            .find(|h| h.provenances()[0].source() == agent_address.clone())
            .map(|h| h.timestamp().clone())
            .or(Some(Iso8601::from(0)))
            .unwrap()
    };

    non_agent_links.sort_by(|link_a, link_b| {
        let timestamp_a = first_timestamp(link_a.clone());
        let timestamp_b = first_timestamp(link_b.clone());
        timestamp_a.cmp(&timestamp_b)
    });

    match hdk::get_entry(&non_agent_links[0].address)? {
        Some(entry) => match Attestation::from_entry(&entry) {
            Some(attestation) => Ok((Some(attestation), non_agent_links.len())),
            None => Err(ZomeApiError::from(String::from(
                "Entry retrieved was not an attestation",
            ))),
        },
        None => Err(ZomeApiError::from(String::from(
            "Could not get attestation when it should be ",
        ))),
    }
}

/**
 * Validates that the given headers are consistent with their transaction and agents
 */
pub fn validate_transaction_headers(chain_headers: &Vec<ChainHeader>) -> ZomeApiResult<()> {
    if chain_headers.len() != 2 {
        return Err(ZomeApiError::from(format!(
            "There are {:?} transaction headers, but only two should exist",
            chain_headers.len()
        )));
    }

    let transaction_address = chain_headers[0].entry_address();

    if !chain_headers
        .iter()
        .all(|h| h.entry_address() == transaction_address)
    {
        return Err(ZomeApiError::from(format!(
            "Transaction headers contain different entry addresses: {:?}",
            chain_headers
        )));
    }
    let offer = offer::query_offer(&transaction_address)?;

    let agent_addresses: Vec<Address> = chain_headers
        .iter()
        .map(|h| h.provenances()[0].source())
        .collect();

    if !agent_addresses.contains(&offer.transaction.creditor_address)
        || !agent_addresses.contains(&offer.transaction.debtor_address)
    {
        return Err(ZomeApiError::from(format!(
            "A transaction header is missing for one of the parties: headers {:?}, transaction: {:?}",
            chain_headers, offer.transaction
        )));
    }

    Ok(())
}
