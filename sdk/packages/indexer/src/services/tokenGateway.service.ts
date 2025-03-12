// import { SUPPORTED_ASSETS_CONTRACT_ADDRESSES } from "@/constants";
// // import { TokenGatewayAbi__factory } from "@/types/contracts";
// import PriceHelper from "@/utils/price.helpers";

// export interface IAssetDetails {
//   erc20_address: string;
//   erc6160_address: string;
//   is_erc20: boolean;
//   is_erc6160: boolean;
// }

// export class TokenGatewayService {
//   /**
//    * Get asset details
//    */
//   static async getAssetDetails(
//     contract_address: string,
//     asset_id: string
//   ): Promise<IAssetDetails> {
//     const tokenGatewayContract = TokenGatewayAbi__factory.connect(
//       contract_address,
//       api
//     );

//     const erc20Address = await tokenGatewayContract.erc20(asset_id);
//     const erc6160Address = await tokenGatewayContract.erc6160(asset_id);

//     return {
//       erc20_address: erc20Address,
//       erc6160_address: erc6160Address,
//       is_erc20: erc20Address !== null && erc20Address.trim().length > 0,
//       is_erc6160: erc6160Address !== null && erc6160Address.trim().length > 0,
//     };
//   }

//   /**
//    * Get the USD value of an asset transfer on TokenGateway
//    */
//   static async getUsdValueOfAsset(
//     chain: string,
//     contract_address: string,
//     asset_id: string,
//     amount: bigint
//   ): Promise<bigint> {
//     const assetDetails = await TokenGatewayService.getAssetDetails(
//       contract_address,
//       asset_id
//     );

//     const assetsSupportedForChain = SUPPORTED_ASSETS_CONTRACT_ADDRESSES[chain];

//     // Ensure we have a list of supported assets for the chain
//     if ((assetsSupportedForChain?.length ?? 0) === 0) {
//       logger.info(`Could not get supported assets for chain ${chain}`);
//       return BigInt(0);
//     }

//     const priceFeedDetails = assetsSupportedForChain.find(
//       (asset) =>
//         asset.address.toLowerCase() == assetDetails.erc20_address.toLowerCase()
//     );

//     if (typeof priceFeedDetails == "undefined") {
//       logger.info(
//         `Could not get asset contract address price feed details on chain ${chain} for asset with assetID ${asset_id}`
//       );
//       return BigInt(0);
//     }

//     const priceInUsd = await PriceHelper.getTokenPriceInUsd(priceFeedDetails);
//     return BigInt(priceInUsd * amount);
//   }
// }
