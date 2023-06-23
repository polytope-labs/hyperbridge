use codec::Encode;
use subxt::{tx::TxPayload, Error, Metadata};

/// Implements [`TxPayload`] for extrinsic encoding
pub struct Extrinsic {
    /// The pallet name, used to query the metadata
    pallet_name: String,
    /// The call name
    call_name: String,
    /// The encoded pallet call. Note that this should be the pallet call. Not runtime call
    encoded: Vec<u8>,
}

impl Extrinsic {
    /// Creates a new extrinsic ready to be sent with subxt.
    pub fn new(
        pallet_name: impl Into<String>,
        call_name: impl Into<String>,
        encoded_call: Vec<u8>,
    ) -> Self {
        Extrinsic {
            pallet_name: pallet_name.into(),
            call_name: call_name.into(),
            encoded: encoded_call,
        }
    }
}

impl TxPayload for Extrinsic {
    fn encode_call_data_to(&self, metadata: &Metadata, out: &mut Vec<u8>) -> Result<(), Error> {
        // encode the pallet index
        let pallet = metadata.pallet_by_name_err(&self.pallet_name)?;
        let call_index = pallet
            .call_variant_by_name(&self.call_name)
            .ok_or_else(|| {
                Error::Other(format!(
                    "Can't find {} in pallet {} metadata",
                    self.call_name, self.pallet_name
                ))
            })?
            .index;
        let pallet_index = pallet.index();
        pallet_index.encode_to(out);
        call_index.encode_to(out);

        // copy the encoded call to out
        out.extend_from_slice(&self.encoded);

        Ok(())
    }
}
