//! Work package builder for pixel submissions.

use grey_codec::Encode;
use grey_types::work::{RefinementContext, WorkItem, WorkPackage};
use grey_types::Hash;

use crate::rpc::ContextResult;

/// Build a JAM-encoded work package that writes a single pixel.
pub fn build_pixel_work_package(
    service_id: u32,
    ctx: &ContextResult,
    x: u8,
    y: u8,
    r: u8,
    g: u8,
    b: u8,
) -> Result<Vec<u8>, String> {
    let code_hash = Hash::from_hex(ctx.code_hash.as_deref().ok_or("missing code_hash")?);
    let anchor = Hash::from_hex(&ctx.anchor);
    let state_root = Hash::from_hex(&ctx.state_root);
    let beefy_root = Hash::from_hex(&ctx.beefy_root);

    let context = RefinementContext {
        anchor,
        state_root,
        beefy_root,
        lookup_anchor: anchor,
        lookup_anchor_timeslot: ctx.slot,
        prerequisites: vec![],
    };

    let item = WorkItem {
        service_id,
        code_hash,
        gas_limit: 5_000_000,
        accumulate_gas_limit: 1_000_000,
        exports_count: 0,
        payload: vec![x, y, r, g, b],
        imports: vec![],
        extrinsics: vec![],
    };

    let wp = WorkPackage {
        auth_code_host: service_id,
        auth_code_hash: code_hash,
        context,
        authorization: vec![],
        authorizer_config: vec![],
        items: vec![item],
    };

    Ok(wp.encode())
}
