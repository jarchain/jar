//! Service accounts, accumulation, and refinement (Sections 9, 12, 14).
//!
//! Services are the core computational units in JAM, analogous to smart contracts.
//! Each service has:
//! - Code (split into Refine and Accumulate entry points)
//! - Storage (key-value dictionary)
//! - Storage quota (managed by privileged quota service)
//! - Preimage lookups
//!
//! Coinless design: no balance field. Storage limits enforced by quota.
//! See docs/ideas/coinless-storage-quota.md.

pub mod accumulation;

use grey_types::state::ServiceAccount;

/// Check if a service can afford the given storage footprint.
/// Returns true if items and bytes are within the service's quota.
pub fn can_afford_storage(account: &ServiceAccount, items: u64, bytes: u64) -> bool {
    items <= account.quota_items && bytes <= account.quota_bytes
}

/// Create a new empty service account with the given code hash.
/// New accounts start with zero quota — the quota service must grant quota.
pub fn new_service_account(
    code_hash: grey_types::Hash,
    min_accumulate_gas: javm::Gas,
    min_on_transfer_gas: javm::Gas,
) -> ServiceAccount {
    ServiceAccount {
        code_hash,
        quota_items: 0,
        min_accumulate_gas,
        min_on_transfer_gas,
        storage: Default::default(),
        preimage_lookup: Default::default(),
        preimage_info: Default::default(),
        quota_bytes: 0,
        total_footprint: 0,
        accumulation_counter: 0,
        last_accumulation: 0,
        last_activity: 0,
        preimage_count: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use grey_types::Hash;

    #[test]
    fn test_quota_check() {
        let mut account = new_service_account(Hash::ZERO, 0, 0);
        // Zero quota — nothing allowed
        assert!(!can_afford_storage(&account, 1, 0));
        assert!(!can_afford_storage(&account, 0, 1));
        assert!(can_afford_storage(&account, 0, 0));

        // Set quota
        account.quota_items = 10;
        account.quota_bytes = 1000;
        assert!(can_afford_storage(&account, 10, 1000));
        assert!(!can_afford_storage(&account, 11, 1000));
        assert!(!can_afford_storage(&account, 10, 1001));
    }
}
