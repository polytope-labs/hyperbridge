// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import "forge-std/Script.sol";
import "openzeppelin/utils/Strings.sol";
import {ERC6160Ext20} from "ERC6160/tokens/ERC6160Ext20.sol";

import {
    TokenGateway,
    Asset,
    TokenGatewayParamsExt,
    TokenGatewayParams,
    AssetFees,
    SetAsset
} from "../src/modules/TokenGateway.sol";
import {TokenFaucet} from "../src/modules/TokenFaucet.sol";
import {PingModule} from "../examples/PingModule.sol";
import {CrossChainMessenger} from "../examples/CrossChainMessenger.sol";
import {StateMachine} from "ismp/StateMachine.sol";

contract DeployScript is Script {
    bytes32 public salt = keccak256(bytes(vm.envString("VERSION")));

    address public SEPOLIA_HOST = 0x9DF353352b469782AB1B0F2CbBFEC41bF1FDbDb3;
    address public ARB_SEPOLIA_HOST = 0x424e6971EB1C693cf4296d4bdb42aa0F32a0dd9e;
    address public OP_SEPOLIA_HOST = 0x1B58A47e61Ca7604b634CBB00b4e275cCd7c9E95;
    address public BASE_SEPOLIA_HOST = 0x4c876500A13cc3825D343b5Ac791d3A4913bF14f;
    address public BSC_TESTNET_HOST = 0x022DDE07A21d8c553978b006D93CDe68ac83e677;

    bytes32 public constant MINTER_ROLE = keccak256("MINTER ROLE");
    bytes32 public constant BURNER_ROLE = keccak256("BURNER ROLE");

    function run() external {
        address admin = vm.envAddress("ADMIN");
        bytes32 privateKey = vm.envBytes32("PRIVATE_KEY");
        string memory host = vm.envString("HOST");
        // todo:
        address uniRouter = address(1);
        address callDispatcher = address(1);

        if (Strings.equal(host, "sepolia") || Strings.equal(host, "ethereum")) {
            vm.startBroadcast(uint256(privateKey));
            deployGateway(SEPOLIA_HOST, admin, uniRouter, callDispatcher);
        } else if (Strings.equal(host, "arbitrum-sepolia")) {
            vm.startBroadcast(uint256(privateKey));
            deployGateway(ARB_SEPOLIA_HOST, admin, uniRouter, callDispatcher);
        } else if (Strings.equal(host, "optimism-sepolia")) {
            vm.startBroadcast(uint256(privateKey));
            deployGateway(OP_SEPOLIA_HOST, admin, uniRouter, callDispatcher);
        } else if (Strings.equal(host, "base-sepolia")) {
            vm.startBroadcast(uint256(privateKey));
            deployGateway(BASE_SEPOLIA_HOST, admin, uniRouter, callDispatcher);
        } else if (Strings.equal(host, "bsc-testnet")) {
            vm.startBroadcast(uint256(privateKey));
            deployGateway(BSC_TESTNET_HOST, admin, uniRouter, callDispatcher);
        }
    }

    function deployMessenger(address host, address admin) public {
        CrossChainMessenger c = new CrossChainMessenger{salt: salt}(admin);
        c.setIsmpHost(host);
    }

    function deployGateway(address host, address admin, address uniRouter, address callDispatcher) public {
        uint256 _paraId = vm.envUint("PARA_ID");

        ERC6160Ext20 feeToken = new ERC6160Ext20{salt: salt}(admin, "Hyperbridge USD", "USD.h");
        feeToken.mint(0x276b41950829E5A7B179ba03B758FaaE9A8d7C41, 1000000000 * 1e18);

        // grant the token faucet
        TokenFaucet faucet = new TokenFaucet{salt: salt}(address(feeToken));
        feeToken.grantRole(MINTER_ROLE, address(faucet));

        TokenGateway gateway = new TokenGateway{salt: salt}(admin);
        feeToken.grantRole(MINTER_ROLE, address(gateway));
        feeToken.grantRole(BURNER_ROLE, address(gateway));

        SetAsset[] memory assets = new SetAsset[](1);
        assets[0] = SetAsset({
            erc20: address(0),
            erc6160: address(feeToken),
            name: "Hyperbridge USD",
            symbol: "USD.h",
            beneficiary: address(0),
            initialSupply: 0,
            fees: AssetFees({
                protocolFeePercentage: 100, // 0.1
                relayerFeePercentage: 300 // 0.3
            })
        });

        gateway.init(
            TokenGatewayParamsExt({
                params: TokenGatewayParams({
                    host: host,
                    uniswapV2: uniRouter,
                    dispatcher: callDispatcher,
                    erc20NativeToken: address(0)
                }),
                assets: assets
            })
        );
    }
}
