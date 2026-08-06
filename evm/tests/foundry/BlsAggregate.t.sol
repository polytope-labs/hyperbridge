// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.30;

import {Test} from "forge-std/Test.sol";
import {BlsAggregate} from "../../src/consensus/BlsAggregate.sol";
import {BlsBeefy} from "../../src/consensus/BlsBeefy.sol";
import {BlsSigner} from "../../src/consensus/Types.sol";

/**
 * @title Cross-language check of the aggregate BLS pairing.
 *
 * @notice Every value here comes from `bls_eip2537_fixture` in the Rust verifier, generated with
 * the same `w3f-bls` code path substrate's BEEFY signers use. That test asserts the aggregate
 * verifies in Rust before emitting anything, so a disagreement here is unambiguously a fault on
 * the EVM side.
 *
 * Needs EIP-2537:
 *   FOUNDRY_PROFILE=bls forge test --match-contract BlsAggregateTest -vv
 */
contract BlsAggregateTest is Test {
    bytes constant MESSAGE = "beefy-bls-aggregate-fixture";

    /// Aggregate signature over MESSAGE by all three signers, G1.
    bytes constant SIG_X =
        hex"16d4d9d336cc264d0f9ddc9086bb5ce441d98896b441253e4d2087aa0fb93a21e44d074d9ac98f525aac914608b001c0";
    bytes constant SIG_Y =
        hex"011ae59b33a10da832193b8ec84bb52958d92c8acf571adad597856db4d40dd5455e1c8a5a59ae1af2afee7e8b36bef6";

    /// The sum of the three public keys, G2. `sumG2` must reproduce this.
    bytes constant AGG_X_C0 =
        hex"05d4a0f9c007ba1cbe0d2b79006d52f8fdb417cb17e0b4a37740b72b2459ec4d030be125b47692d9f8bbd9e937291831";
    bytes constant AGG_X_C1 =
        hex"120a8de45bee1aab433b63d9859f40b560f0b923ecc23bbd2bf41e90290c4ba97b31137e70ab3e18a0e3616c340cb584";
    bytes constant AGG_Y_C0 =
        hex"0e5e2bce96aa5ae0b57f3da3add49b35c60ecc276eec38940a5a0cad2a7ddf5555e954bce53d5b2c34b2b4824742e60d";
    bytes constant AGG_Y_C1 =
        hex"0530f71228f36639e9d197e48a3d27697d71acca411533eef32d0d602e4d8fd72f52edfff40ada6b634797bfa3b1a19f";

    /// Build the 64 byte field element EIP-2537 expects: 16 zero bytes then the 48 byte value.
    function _fp(bytes memory value) private pure returns (bytes memory) {
        return bytes.concat(new bytes(16), value);
    }

    function _g1(bytes memory x, bytes memory y) private pure returns (bytes memory) {
        return bytes.concat(_fp(x), _fp(y));
    }

    function _g2(bytes memory xc0, bytes memory xc1, bytes memory yc0, bytes memory yc1)
        private
        pure
        returns (bytes memory)
    {
        return bytes.concat(_fp(xc0), _fp(xc1), _fp(yc0), _fp(yc1));
    }

    function _signers() private pure returns (bytes[] memory keys) {
        keys = new bytes[](3);
        keys[0] = _g2(
            hex"0eb912203efe065b9d9844025f9a85a43fdb21f4c8c0f31b39cc3bedcdbecce8ab3567f9cadbe965adebb7dd4a081ec1",
            hex"14168206974b9223cc95e6e1f279f9d10e526aa172b5bd15b101b6f4997e2038ebcb02bfee1bca54f428162e17ade003",
            hex"0dbe0ed3b59dbf3c217e879f885df4fce29af686888e77e984b69d6a07fe90b2a1acebc49d6a196c90a9307be82bb9c4",
            hex"16bd04776624eab548fc58aeac9da7f75a618206620c9119d517b983b659ca196c2a0761e09a1ee6df87e1cd78bdd4d5"
        );
        keys[1] = _g2(
            hex"065a060f78222114141a2544d6207ebfe7784788e6310b3a58cebc36df948e387853ce8492c4c8f9d423ebabc6feb027",
            hex"026bc8c3c41fb3bb78a7d97855098d8b32ec546433c4c523f62c471b75d89823e2485ff79f3c203961b62c9b8b08e4d1",
            hex"0db39f4b44160911c089c2cf24621f3e1df13561ced1640ddafa4c885d0a80b3428156c709f375e00fdb230b5c109e46",
            hex"00b7e2a03248e77666c8e12d4eaa31d451c477209a178134afd53ebcc701842206b2b7613ed4807f4600ce212c6679e8"
        );
        keys[2] = _g2(
            hex"0c2410d03233711f03a7242fd3a8bb141125ac84a07813d8cce6e366038f129d3ef348fc7dd14682e3db28a920d2c748",
            hex"008cd3cbf5e8dd6aca7d5fb78061c360910691c3797bcd95a42cf5a45b0612e8555f6e118788872af47d231954914282",
            hex"15c168dc4a702011de9bded446897040c88e51831293bbcc691fe85a7f42e6cb454d4931fe9348fa310f455a21ef47ae",
            hex"08600a717fc096a40eeb7dba5194779267123c9fef22e3ebe85c34c1aa74928a89c211d9bfa7dc97a296fc7550acc58e"
        );
    }

    /// `G2ADD` over the signers must land on the aggregate key Rust computed.
    function test_sum_g2_matches_rust() public view {
        bytes memory summed = BlsAggregate.sumG2(_signers());
        assertEq(summed, _g2(AGG_X_C0, AGG_X_C1, AGG_Y_C0, AGG_Y_C1), "aggregate key differs from Rust");
    }

    /// The whole thing: one pairing check over the aggregate.
    function test_verify_aggregate() public view {
        assertTrue(BlsAggregate.verify(MESSAGE, _g1(SIG_X, SIG_Y), _signers()), "aggregate signature should verify");
    }

    /// A different message must not verify, or the check proves nothing.
    function test_rejects_wrong_message() public view {
        assertFalse(
            BlsAggregate.verify("a different commitment", _g1(SIG_X, SIG_Y), _signers()),
            "signature over another message must not verify"
        );
    }

    /// Dropping a signer from the key set breaks the aggregate, so a validator cannot be credited
    /// with a signature they did not produce.
    function test_rejects_missing_signer() public view {
        bytes[] memory all = _signers();
        bytes[] memory subset = new bytes[](2);
        subset[0] = all[0];
        subset[1] = all[1];

        assertFalse(
            BlsAggregate.verify(MESSAGE, _g1(SIG_X, SIG_Y), subset),
            "aggregate must not verify against a subset of the signers"
        );
    }

    /// Root of the authority set tree: four per-authority leaves, then the BLS commitment.
    bytes32 constant KEYSET_ROOT = 0x9097854fdde72a6cae144165afd1b881ff201a20acbbbef836f9cdf45a1d85a9;
    /// Root of the tree over the four authorities' compressed BLS keys, the extra leaf's value.
    bytes32 constant BLS_COMMITMENT = 0xd220f3b093a9c3cb95b44e1413e438eb0184b9fe9337591ef13680c44e678a2a;
    /// Opens the BLS commitment as leaf 4 of the five-leaf authority set tree.
    bytes32 constant KEYSET_PROOF_NODE = 0x9ca0bbf4e382871c43e750de6df39704e2604f75b83534dfa710289745b331f7;
    /// Opens signers 0,1,2 against the BLS commitment.
    bytes32 constant PROOF_NODE = 0x97e418e070e3ec967bccc1d0aaea6d787a7b68733451c5320cd32eec2be63df8;

    function _blsSigners() private pure returns (BlsSigner[] memory out) {
        bytes[] memory keys = _signers();
        out = new BlsSigner[](3);
        for (uint256 i = 0; i < 3; ++i) {
            out[i] = BlsSigner({publicKey: keys[i], authorityIndex: i});
        }
    }

    function _authorityProof() private pure returns (bytes32[] memory nodes) {
        nodes = new bytes32[](1);
        nodes[0] = PROOF_NODE;
    }

    function _keysetProof() private pure returns (bytes32[] memory nodes) {
        nodes = new bytes32[](1);
        nodes[0] = KEYSET_PROOF_NODE;
    }

    /// The authority half end to end: threshold, ordering, merkle membership, aggregate pairing.
    /// Reverts on any failure, so reaching the end is the assertion.
    function test_verify_authorities() public {
        new BlsBeefy().verifyAuthorities(MESSAGE, _blsSigners(), _g1(SIG_X, SIG_Y), KEYSET_ROOT, BLS_COMMITMENT, _keysetProof(), _authorityProof(), 4);
    }

    /// Repeating a signer must be rejected before any cryptography runs.
    function test_rejects_duplicate_signer_index() public {
        BlsSigner[] memory signers = _blsSigners();
        signers[2].authorityIndex = 1;

        BlsBeefy client = new BlsBeefy();
        vm.expectRevert(BlsBeefy.InvalidSignerOrdering.selector);
        client.verifyAuthorities(MESSAGE, signers, _g1(SIG_X, SIG_Y), KEYSET_ROOT, BLS_COMMITMENT, _keysetProof(), _authorityProof(), 4);
    }

    /// Two of four is short of the supermajority.
    function test_rejects_sub_supermajority() public {
        BlsSigner[] memory all = _blsSigners();
        BlsSigner[] memory two = new BlsSigner[](2);
        two[0] = all[0];
        two[1] = all[1];

        BlsBeefy client = new BlsBeefy();
        vm.expectRevert(BlsBeefy.SuperMajorityRequired.selector);
        client.verifyAuthorities(MESSAGE, two, _g1(SIG_X, SIG_Y), KEYSET_ROOT, BLS_COMMITMENT, _keysetProof(), _authorityProof(), 4);
    }

    /// Compression must reproduce exactly what `w3f-bls` serialises, or the merkle leaves will not
    /// match the commitment. Signer 0 has the sign bit set (0xb4), signer 2 does not (0x80), so
    /// both branches of the rule are covered.
    function test_compress_matches_w3f_bls() public pure {
        bytes[] memory keys = _signers();

        assertEq(
            BlsAggregate.compressG2(keys[0]),
            hex"b4168206974b9223cc95e6e1f279f9d10e526aa172b5bd15b101b6f4997e2038ebcb02bfee1bca54f428162e17ade003"
            hex"0eb912203efe065b9d9844025f9a85a43fdb21f4c8c0f31b39cc3bedcdbecce8ab3567f9cadbe965adebb7dd4a081ec1",
            "signer 0 compression differs, sign bit set"
        );

        assertEq(
            BlsAggregate.compressG2(keys[2]),
            hex"808cd3cbf5e8dd6aca7d5fb78061c360910691c3797bcd95a42cf5a45b0612e8555f6e118788872af47d231954914282"
            hex"0c2410d03233711f03a7242fd3a8bb141125ac84a07813d8cce6e366038f129d3ef348fc7dd14682e3db28a920d2c748",
            "signer 2 compression differs, sign bit clear"
        );
    }
}
