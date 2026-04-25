import VersoManual
import Jar.Types

open Verso.Genre Manual
open Jar

set_option verso.docstring.allowMissing true

#doc (Manual) "Economic Model" =>

jar1 uses a *coinless* economy. There are no tokens, no balance transfers, and no
storage rent. Instead, storage capacity is governed by quotas - a privileged
*quota service* (chi\_Q) sets per-service limits on storage items and bytes.

This is a fundamental departure from the Gray Paper's balance-based model (gp072
variants), where services must hold sufficient token balance to cover storage
deposit costs. In jar1, the quota service acts as a governance mechanism:
services that exceed their quota cannot write new storage items.

# The EconModel Typeclass

The protocol abstracts over economic models via the `EconModel` typeclass,
which defines operations for storage affordability checks, transfer handling,
service creation debits, and quota management.

{docstring EconModel}

# Quota-Based Economy (jar1)

{docstring QuotaEcon}

{docstring QuotaTransfer}

In the quota model, `canAffordStorage` checks whether the service's current item
count and byte count are within the quota limits. Transfers carry no token amount -
`QuotaTransfer` is a unit type for pure message-passing. The `setQuota` operation
(host call 28, available to the privileged quota service) adjusts a service's
storage limits.

This means jar1 removes the Gray Paper assumption that every state change must be
paid for with a native token balance. The economic question becomes: *is this
service still within the storage budget granted to it by governance?* Once a
service exhausts its quota, writes fail even if the service is otherwise valid.

The model is still economic in the broad sense - it governs scarce storage and
service creation - but it does so with protocol-administered limits rather than
price-denominated deposits. In practice, quota replaces balance as the resource
that accumulation checks before accepting growth in state.

# Balance-Based Economy (gp072)

For reference, the Gray Paper variants use a token-based model where services must
hold sufficient balance to cover storage deposit costs.

{docstring BalanceEcon}

{docstring BalanceTransfer}

Under the balance model, storage growth consumes economic slack from the account
itself. New services must be funded at creation time, ordinary transfers move
token balance between services, and storage affordability is evaluated against a
minimum-balance threshold derived from item count and byte footprint.

# Service Accounts

Service accounts hold code, storage, preimages, and economic state. The economic
fields are parameterized by the variant's `EconType`.

{docstring ServiceAccount}

The same service-account structure supports both models. What changes across
variants is the concrete meaning of the `econ` field: balance and gratis storage
offsets in gp072, quota item and byte ceilings in jar1.

# Privileged Services

Certain services have special protocol roles. In jar1, the `quotaService` field
identifies the service authorized to call the `set\_quota` host call.

{docstring PrivilegedServices}

Because jar1 is coinless, privileged services become more important. The quota
service effectively decides which services are allowed to keep growing their
state, making quota assignment part of the protocol's governance surface rather
than an emergent market price.

# Deferred Transfers

Transfers between services are deferred to accumulation. In jar1, transfers carry
no token amount - they are pure inter-service messages with a memo and gas budget.

This is an important distinction: jar1 removes *token transfer* semantics, not
inter-service communication. Deferred transfers still exist because services need
an asynchronous way to send intent, memo data, and execution gas into another
service's on-transfer handler during accumulation.

{docstring DeferredTransfer}
