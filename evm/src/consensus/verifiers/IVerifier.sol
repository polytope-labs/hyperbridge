// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.0;

interface IVerifier {
    /**
     * @notice Verify a Ultra Plonk proof
     * @param _proof - The serialized proof
     * @param _publicInputs - An array of the public inputs
     * @return True if proof is valid, reverts otherwise
     */
    function verify(bytes calldata _proof, bytes32[] calldata _publicInputs) external view returns (bool);
}
