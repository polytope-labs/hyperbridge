// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import {IIsmpModule} from "ismp/IIsmpModule.sol";
import {PostRequest, PostResponse, GetResponse, GetRequest} from "ismp/IIsmp.sol";

abstract contract BaseIsmpModule is IIsmpModule {
    function onAccept(PostRequest calldata request) external virtual {
        revert("IsmpModule doesn't expect Post requests");
    }

    function onPostRequestTimeout(PostRequest memory) external view virtual {
        revert("IsmpModule doesn't emit Post requests");
    }

    function onPostResponse(PostResponse memory) external view virtual {
        revert("IsmpModule doesn't emit Post responses");
    }

    function onPostResponseTimeout(PostResponse memory) external view virtual {
        revert("IsmpModule doesn't emit Post responses");
    }

    function onGetResponse(GetResponse memory) external view virtual {
        revert("IsmpModule doesn't emit Get requests");
    }

    function onGetTimeout(GetRequest memory) external view virtual {
        revert("IsmpModule doesn't emit Get requests");
    }
}
