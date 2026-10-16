-- Governance tables as the indexer creates them, with the referenda, motions and votes it indexed
-- from Nexus blocks 12135052 to 12335491, for the API's integration test.
CREATE SCHEMA governance;

CREATE TABLE governance._metadata (
    key character varying(255) NOT NULL,
    value jsonb,
    "createdAt" timestamp with time zone NOT NULL,
    "updatedAt" timestamp with time zone NOT NULL
);
CREATE TABLE governance.governance_delegations (
    id text NOT NULL,
    delegator text NOT NULL,
    target text NOT NULL,
    track integer NOT NULL,
    conviction integer NOT NULL,
    balance numeric NOT NULL,
    active boolean NOT NULL,
    from_block integer NOT NULL,
    to_block integer
);
CREATE TABLE governance.governance_motion_events (
    id text NOT NULL,
    motion_id text NOT NULL,
    name text NOT NULL,
    args text,
    block_number integer NOT NULL,
    block_hash text NOT NULL,
    extrinsic_index integer,
    event_index integer NOT NULL,
    "timestamp" timestamp without time zone NOT NULL
);
CREATE TABLE governance.governance_motions (
    id text NOT NULL,
    index integer NOT NULL,
    hash text NOT NULL,
    proposer text NOT NULL,
    threshold integer NOT NULL,
    call text,
    ayes jsonb NOT NULL,
    nays jsonb NOT NULL,
    "end" integer,
    state text NOT NULL,
    is_final boolean NOT NULL,
    result text,
    proposed_block integer NOT NULL,
    proposed_at timestamp without time zone NOT NULL,
    updated_block integer NOT NULL
);
CREATE TABLE governance.governance_preimages (
    id text NOT NULL,
    length integer,
    data text,
    status text NOT NULL,
    noted_block integer,
    updated_block integer NOT NULL
);
CREATE TABLE governance.governance_referendum_events (
    id text NOT NULL,
    referendum_id text NOT NULL,
    name text NOT NULL,
    args text,
    block_number integer NOT NULL,
    block_hash text NOT NULL,
    extrinsic_index integer,
    event_index integer NOT NULL,
    "timestamp" timestamp without time zone NOT NULL,
    changes_state boolean
);
CREATE TABLE governance.governance_referendums (
    id text NOT NULL,
    index integer NOT NULL,
    track integer NOT NULL,
    proposer text,
    proposal text NOT NULL,
    proposal_hash text,
    call text,
    state text NOT NULL,
    is_final boolean NOT NULL,
    ayes numeric NOT NULL,
    nays numeric NOT NULL,
    support numeric NOT NULL,
    electorate numeric,
    info text,
    enactment_task text,
    enactment_result text,
    submitted_block integer NOT NULL,
    submitted_at timestamp without time zone NOT NULL,
    updated_block integer NOT NULL
);
CREATE TABLE governance.governance_tally_snapshots (
    id text NOT NULL,
    referendum_id text NOT NULL,
    ayes numeric NOT NULL,
    nays numeric NOT NULL,
    support numeric NOT NULL,
    electorate numeric NOT NULL,
    block_number integer NOT NULL,
    "timestamp" timestamp without time zone NOT NULL
);
CREATE TABLE governance.governance_tracks (
    id text NOT NULL,
    name text NOT NULL,
    info text NOT NULL,
    updated_block integer NOT NULL
);
CREATE TABLE governance.governance_treasury_spends (
    id text NOT NULL,
    index integer NOT NULL,
    asset_kind text NOT NULL,
    amount numeric NOT NULL,
    beneficiary text NOT NULL,
    valid_from integer NOT NULL,
    expire_at integer NOT NULL,
    state text NOT NULL,
    payment_id text,
    approved_block integer NOT NULL,
    approved_at timestamp without time zone NOT NULL,
    updated_block integer NOT NULL
);
CREATE TABLE governance.governance_vote_calls (
    id text NOT NULL,
    account text NOT NULL,
    referendum_index integer NOT NULL,
    vote text,
    removed boolean NOT NULL,
    block_number integer NOT NULL,
    block_hash text NOT NULL,
    extrinsic_index integer,
    event_index integer NOT NULL,
    "timestamp" timestamp without time zone NOT NULL
);
CREATE TABLE governance.governance_votes (
    id text NOT NULL,
    referendum_index integer NOT NULL,
    track integer NOT NULL,
    account text NOT NULL,
    kind text NOT NULL,
    aye boolean,
    conviction integer,
    balance numeric NOT NULL,
    aye_balance numeric,
    nay_balance numeric,
    abstain_balance numeric,
    delegated_votes numeric NOT NULL,
    delegated_capital numeric NOT NULL,
    block_number integer NOT NULL
);
CREATE TABLE governance.governance_whitelisted_calls (
    id text NOT NULL,
    whitelisted_block integer NOT NULL,
    removed_block integer,
    dispatched_block integer,
    dispatch_result text
);

INSERT INTO governance.governance_motions VALUES
	('65', 65, '0xe526eafaa7faf08bdfc617829ccbc7958c41ffc4e6f43a8d488fe95ebdc8be49', '15PCbSw1gdtCG9fp1AdLVEQnovPh7LLYWN9krTDAZYhHirSY', 2, '{"callIndex":"0x5300","section":"whitelist","method":"whitelistCall","args":[{"name":"callHash","type":"H256","value":"0x3437c169b25cdf20c03e0241c5204151ffc478c4c8b2878f6fffe27052f5f029"}]}', '["15PCbSw1gdtCG9fp1AdLVEQnovPh7LLYWN9krTDAZYhHirSY", "1333zsMafds2sKAr8nG3zwXTCHPYv2Nm6CRgakpu6YVGt7nM"]', '[]', 12285416, 'Executed', true, '{"ok":null}', 12249416, '2026-10-09 17:03:24', 12249554),
	('66', 66, '0xce2e15bd38b7027372723a0a4372a95e2fdd7c559d8de3476cbd778aef8dc6ac', '15PCbSw1gdtCG9fp1AdLVEQnovPh7LLYWN9krTDAZYhHirSY', 2, '{"callIndex":"0x5300","section":"whitelist","method":"whitelistCall","args":[{"name":"callHash","type":"H256","value":"0x774793b5b7b85afd9f10cd6207f363dba366c17357112eb11092253377c4be9c"}]}', '["15PCbSw1gdtCG9fp1AdLVEQnovPh7LLYWN9krTDAZYhHirSY", "1333zsMafds2sKAr8nG3zwXTCHPYv2Nm6CRgakpu6YVGt7nM"]', '[]', 12295118, 'Executed', true, '{"ok":null}', 12259118, '2026-10-10 09:32:36', 12259265);
INSERT INTO governance.governance_motion_events VALUES
	('12249416-6', '65', 'Proposed', '{"account":"15PCbSw1gdtCG9fp1AdLVEQnovPh7LLYWN9krTDAZYhHirSY","proposalIndex":65,"proposalHash":"0xe526eafaa7faf08bdfc617829ccbc7958c41ffc4e6f43a8d488fe95ebdc8be49","threshold":2}', 12249416, '0x53c0f7a4d746207ec5e54e9c6ef6f071221ee32ce83ffedb5363b3d1c8fbb8e2', 2, 6, '2026-10-09 17:03:24'),
	('12249439-6', '65', 'Voted', '{"account":"15PCbSw1gdtCG9fp1AdLVEQnovPh7LLYWN9krTDAZYhHirSY","proposalHash":"0xe526eafaa7faf08bdfc617829ccbc7958c41ffc4e6f43a8d488fe95ebdc8be49","voted":true,"yes":1,"no":0}', 12249439, '0xf0df2fc06a47e6e5716d3d99ccbc6400b4594743ff39f852166d2598e80c47d3', 2, 6, '2026-10-09 17:05:42'),
	('12249536-6', '65', 'Voted', '{"account":"1333zsMafds2sKAr8nG3zwXTCHPYv2Nm6CRgakpu6YVGt7nM","proposalHash":"0xe526eafaa7faf08bdfc617829ccbc7958c41ffc4e6f43a8d488fe95ebdc8be49","voted":true,"yes":2,"no":0}', 12249536, '0x57cef9ac5c5143979c2a288c5a8d4927998113d7bcbe2ef56d78e1e054a10b00', 2, 6, '2026-10-09 17:15:48'),
	('12249554-6', '65', 'Closed', '{"proposalHash":"0xe526eafaa7faf08bdfc617829ccbc7958c41ffc4e6f43a8d488fe95ebdc8be49","yes":2,"no":0}', 12249554, '0x2c0e7ae3fffb38dbd682dc842e36c02eff8876e26b25b43ce8c399d9e52f1826', 2, 6, '2026-10-09 17:17:36'),
	('12249554-7', '65', 'Approved', '{"proposalHash":"0xe526eafaa7faf08bdfc617829ccbc7958c41ffc4e6f43a8d488fe95ebdc8be49"}', 12249554, '0x2c0e7ae3fffb38dbd682dc842e36c02eff8876e26b25b43ce8c399d9e52f1826', 2, 7, '2026-10-09 17:17:36'),
	('12249554-10', '65', 'Executed', '{"proposalHash":"0xe526eafaa7faf08bdfc617829ccbc7958c41ffc4e6f43a8d488fe95ebdc8be49","result":{"ok":null}}', 12249554, '0x2c0e7ae3fffb38dbd682dc842e36c02eff8876e26b25b43ce8c399d9e52f1826', 2, 10, '2026-10-09 17:17:36'),
	('12259118-6', '66', 'Proposed', '{"account":"15PCbSw1gdtCG9fp1AdLVEQnovPh7LLYWN9krTDAZYhHirSY","proposalIndex":66,"proposalHash":"0xce2e15bd38b7027372723a0a4372a95e2fdd7c559d8de3476cbd778aef8dc6ac","threshold":2}', 12259118, '0x341ce7dd1adf65948f6d6283ab1476e6df978420871515fd68063ca50ad1bac3', 2, 6, '2026-10-10 09:32:36'),
	('12259145-6', '66', 'Voted', '{"account":"15PCbSw1gdtCG9fp1AdLVEQnovPh7LLYWN9krTDAZYhHirSY","proposalHash":"0xce2e15bd38b7027372723a0a4372a95e2fdd7c559d8de3476cbd778aef8dc6ac","voted":true,"yes":1,"no":0}', 12259145, '0x2f58f9a8edbbd7343020b0557d1577700f5fadab0c7698cde840b7d322b3c4d8', 2, 6, '2026-10-10 09:35:18'),
	('12259204-6', '66', 'Voted', '{"account":"1333zsMafds2sKAr8nG3zwXTCHPYv2Nm6CRgakpu6YVGt7nM","proposalHash":"0xce2e15bd38b7027372723a0a4372a95e2fdd7c559d8de3476cbd778aef8dc6ac","voted":true,"yes":2,"no":0}', 12259204, '0x35fcbe572fac88d69897bdc5245ed75312ffa3669ae5e0cbd817cbe18b046f8a', 2, 6, '2026-10-10 09:41:18'),
	('12259265-6', '66', 'Closed', '{"proposalHash":"0xce2e15bd38b7027372723a0a4372a95e2fdd7c559d8de3476cbd778aef8dc6ac","yes":2,"no":0}', 12259265, '0xc9d88a5d530b60e4d7219d9b68aa0cac5f234b06a93c66c952c1ce5ab898c6f5', 2, 6, '2026-10-10 09:47:24'),
	('12259265-7', '66', 'Approved', '{"proposalHash":"0xce2e15bd38b7027372723a0a4372a95e2fdd7c559d8de3476cbd778aef8dc6ac"}', 12259265, '0xc9d88a5d530b60e4d7219d9b68aa0cac5f234b06a93c66c952c1ce5ab898c6f5', 2, 7, '2026-10-10 09:47:24'),
	('12259265-10', '66', 'Executed', '{"proposalHash":"0xce2e15bd38b7027372723a0a4372a95e2fdd7c559d8de3476cbd778aef8dc6ac","result":{"ok":null}}', 12259265, '0xc9d88a5d530b60e4d7219d9b68aa0cac5f234b06a93c66c952c1ce5ab898c6f5', 2, 10, '2026-10-10 09:47:24');
INSERT INTO governance.governance_preimages VALUES
	('0x07c55346395111f0644752e6192222aaf11daf1e27a0e54f380a05a554cc3a88', NULL, NULL, 'Cleared', NULL, 12249405),
	('0x9ad837ef4b6a5268e6a9dcb7128c40ba61c4c305be5d15a07ebca9cae357a3d8', NULL, NULL, 'Cleared', NULL, 12249408),
	('0x083b73cae4643c9ef0393becd1df34f50bd965852c46b4f805f632a47e933b38', NULL, NULL, 'Cleared', NULL, 12249410),
	('0x3437c169b25cdf20c03e0241c5204151ffc478c4c8b2878f6fffe27052f5f029', 34, '0x0009d1576e0312bcf61776c56112368dafaaec30ba7379fa51904ad195a030979889', 'Cleared', 12249398, 12259083),
	('0xc1f3d5a7d3c9e26d476ec8de08578923a577ba1fad01c4d617d4293a129981f6', 43, '0x53023437c169b25cdf20c03e0241c5204151ffc478c4c8b2878f6fffe27052f5f0292200000062c9e41900', 'Cleared', 12249465, 12259086),
	('0x774793b5b7b85afd9f10cd6207f363dba366c17357112eb11092253377c4be9c', 1298, '0x0402545e0b0001000000c2094f7094623fba8751c58061dd9fe26ce5e59a108fd3ab805e0b00b1a40000c2094f7094623fba8751c58061dd9fe26ce5e59a108fd3ab805e0b0005210000c2094f7094623fba8751c58061dd9fe26ce5e59a108fd3ab805e0b0038000000c2094f7094623fba8751c58061dd9fe26ce5e59a108fd3ab805e0b0089000000c2094f7094623fba8751c58061dd9fe26ce5e59a108fd3ab805e13000100000011024f1ef286000000000000000000000000f159b72f168bceb982e591176d81ca6568d008dd000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000048fd3ab80000000000000000000000000000000000000000000000000000000005e1300b1a4000011024f1ef286000000000000000000000000f159b72f168bceb982e591176d81ca6568d008dd000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000048fd3ab80000000000000000000000000000000000000000000000000000000005e13000521000011024f1ef286000000000000000000000000f159b72f168bceb982e591176d81ca6568d008dd000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000048fd3ab80000000000000000000000000000000000000000000000000000000005e13003800000011024f1ef286000000000000000000000000f159b72f168bceb982e591176d81ca6568d008dd000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000048fd3ab80000000000000000000000000000000000000000000000000000000005e13008900000011024f1ef286000000000000000000000000f159b72f168bceb982e591176d81ca6568d008dd000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000048fd3ab80000000000000000000000000000000000000000000000000000000005e1300431b0f1911024f1ef286000000000000000000000000f159b72f168bceb982e591176d81ca6568d008dd000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000048fd3ab80000000000000000000000000000000000000000000000000000000005e150001000000045668cd0b956fe12955514496b8a3dc4971402f80015e1500b1a40000045668cd0b956fe12955514496b8a3dc4971402f80015e150005210000045668cd0b956fe12955514496b8a3dc4971402f80015e150038000000045668cd0b956fe12955514496b8a3dc4971402f80015e150089000000045668cd0b956fe12955514496b8a3dc4971402f80013501000a00000000000000000001ffffffffffffffffffffffffffffffff000000003501006400000000000000000001ffffffffffffffffffffffffffffffff000000003501004c07000000000000000001ffffffffffffffffffffffffffffffff00000000290200474e4f53474e4f30ffffffffffffffff00002902004f5054434f505430ffffffffffffffff0000', 'Requested', 12259004, 12259265),
	('0xf063b48b66892b3a94bc6c83889470e461a69483fc2412b7fdad8778795eef86', 48, '0x5302774793b5b7b85afd9f10cd6207f363dba366c17357112eb11092253377c4be9c12050000077d028e8502ce260400', 'Requested', 12259156, 12259858);
INSERT INTO governance.governance_referendums VALUES
	('79', 79, 1, '15PCbSw1gdtCG9fp1AdLVEQnovPh7LLYWN9krTDAZYhHirSY', '{"lookup":{"hash":"0xc1f3d5a7d3c9e26d476ec8de08578923a577ba1fad01c4d617d4293a129981f6","len":43}}', '0xc1f3d5a7d3c9e26d476ec8de08578923a577ba1fad01c4d617d4293a129981f6', '{"callIndex":"0x5302","section":"whitelist","method":"dispatchWhitelistedCall","args":[{"name":"callHash","type":"H256","value":"0x3437c169b25cdf20c03e0241c5204151ffc478c4c8b2878f6fffe27052f5f029"},{"name":"callEncodedLen","type":"u32","value":34},{"name":"callWeightWitness","type":"SpWeightsWeightV2Weight","value":{"refTime":108606040,"proofSize":0}}]}', 'Executed', true, 349483333333333343398, 0, 349483333333333343398, 610412965456601837742, '{"track":1,"origin":{"origins":"WhitelistedCaller"},"proposal":{"lookup":{"hash":"0xc1f3d5a7d3c9e26d476ec8de08578923a577ba1fad01c4d617d4293a129981f6","len":43}},"enactment":{"after":30},"submitted":12249468,"submissionDeposit":{"who":"15PCbSw1gdtCG9fp1AdLVEQnovPh7LLYWN9krTDAZYhHirSY","amount":3000000000},"decisionDeposit":{"who":"1333zsMafds2sKAr8nG3zwXTCHPYv2Nm6CRgakpu6YVGt7nM","amount":"0x0000000000000000058d15e176280000"},"deciding":{"since":12249568,"confirming":12250172},"tally":{"ayes":"0x0000000000000012f20e38064cc47ca6","nays":0,"support":"0x0000000000000012f20e38064cc47ca6"},"inQueue":false,"alarm":[12250172,[12250172,0]]}', '0x88d5b89a59c207c6743b97f439c2658a44f528e8de2400f7064ad7761cfaeb2f', '{"ok":null}', 12249468, '2026-10-09 17:08:36', 12259206),
	('80', 80, 1, '15PCbSw1gdtCG9fp1AdLVEQnovPh7LLYWN9krTDAZYhHirSY', '{"lookup":{"hash":"0xf063b48b66892b3a94bc6c83889470e461a69483fc2412b7fdad8778795eef86","len":48}}', '0xf063b48b66892b3a94bc6c83889470e461a69483fc2412b7fdad8778795eef86', '{"callIndex":"0x5302","section":"whitelist","method":"dispatchWhitelistedCall","args":[{"name":"callHash","type":"H256","value":"0x774793b5b7b85afd9f10cd6207f363dba366c17357112eb11092253377c4be9c"},{"name":"callEncodedLen","type":"u32","value":1298},{"name":"callWeightWitness","type":"SpWeightsWeightV2Weight","value":{"refTime":10830611069,"proofSize":68019}}]}', 'Executed', true, 349483333333333343398, 0, 349483333333333343398, 610525856142653277953, '{"track":1,"origin":{"origins":"WhitelistedCaller"},"proposal":{"lookup":{"hash":"0xf063b48b66892b3a94bc6c83889470e461a69483fc2412b7fdad8778795eef86","len":48}},"enactment":{"after":30},"submitted":12259158,"submissionDeposit":{"who":"15PCbSw1gdtCG9fp1AdLVEQnovPh7LLYWN9krTDAZYhHirSY","amount":3000000000},"decisionDeposit":{"who":"1333zsMafds2sKAr8nG3zwXTCHPYv2Nm6CRgakpu6YVGt7nM","amount":"0x0000000000000000058d15e176280000"},"deciding":{"since":12259258,"confirming":12259858},"tally":{"ayes":"0x0000000000000012f20e38064cc47ca6","nays":0,"support":"0x0000000000000012f20e38064cc47ca6"},"inQueue":false,"alarm":[12259858,[12259858,0]]}', '0x60d77a6d602388d7009a9a38ee31063a1e43e9a6685a418c023387c73f960a51', '{"ok":null}', 12259158, '2026-10-10 09:36:36', 12300863);
INSERT INTO governance.governance_referendum_events VALUES
	('12249468-14', '79', 'Submitted', '{"index":79,"track":1,"proposal":{"lookup":{"hash":"0xc1f3d5a7d3c9e26d476ec8de08578923a577ba1fad01c4d617d4293a129981f6","len":43}}}', 12249468, '0x34e86e3c281110843c7a41928ba398d39a9b6cc90ddfce6f10105c54c90c6cf6', 4, 14, '2026-10-09 17:08:36', true),
	('12249517-20', '79', 'DecisionDepositPlaced', '{"index":79,"who":"1333zsMafds2sKAr8nG3zwXTCHPYv2Nm6CRgakpu6YVGt7nM","amount":"0x0000000000000000058d15e176280000"}', 12249517, '0xfb74a9d478ca5db2f85931c528869aea572deaee923c94bb573c279652c2a656', 5, 20, '2026-10-09 17:13:54', true),
	('12249568-3', '79', 'DecisionStarted', '{"index":79,"track":1,"proposal":{"lookup":{"hash":"0xc1f3d5a7d3c9e26d476ec8de08578923a577ba1fad01c4d617d4293a129981f6","len":43}},"tally":{"ayes":"0x000000000000000ad78ebc5ac6200000","nays":0,"support":"0x000000000000000ad78ebc5ac6200000"}}', 12249568, '0x7601b389cb9157718c693931d04827b252a19c2f47228cee2b7cdd3885c9e282', NULL, 3, '2026-10-09 17:19:00', true),
	('12249572-3', '79', 'ConfirmStarted', '{"index":79}', 12249572, '0x48ab045aae9ce8039e1cb4ddd8f6fbf4320027f3aca77853ec1a684d50ead92e', NULL, 3, '2026-10-09 17:19:24', true),
	('12250172-6', '79', 'Confirmed', '{"index":79,"tally":{"ayes":"0x0000000000000012f20e38064cc47ca6","nays":0,"support":"0x0000000000000012f20e38064cc47ca6"}}', 12250172, '0xe9a8aef7aac2257ac10aeb4e05e85556ec3be5a7783d96f85578bf18c682a352', NULL, 6, '2026-10-09 18:19:48', true),
	('12250202-5', '79', 'Executed', '{"result":{"ok":null}}', 12250202, '0xeb3f62dbd95d20711510f24b68586180cef9d03a27982feb330ca621e3b36259', NULL, 5, '2026-10-09 18:22:48', true),
	('12259158-18', '80', 'Submitted', '{"index":80,"track":1,"proposal":{"lookup":{"hash":"0xf063b48b66892b3a94bc6c83889470e461a69483fc2412b7fdad8778795eef86","len":48}}}', 12259158, '0xd67932b58acdaeca0186d60c514d8adf04acbd5736daf6959247431f43d82191', 4, 18, '2026-10-10 09:36:36', true),
	('12259206-17', '79', 'DecisionDepositRefunded', '{"index":79,"who":"1333zsMafds2sKAr8nG3zwXTCHPYv2Nm6CRgakpu6YVGt7nM","amount":"0x0000000000000000058d15e176280000"}', 12259206, '0x510b1903c238c884a99e9f6374052b9207439e9d8293faaa0bbae066b5cc3a95', 4, 17, '2026-10-10 09:41:30', false),
	('12259210-22', '80', 'DecisionDepositPlaced', '{"index":80,"who":"1333zsMafds2sKAr8nG3zwXTCHPYv2Nm6CRgakpu6YVGt7nM","amount":"0x0000000000000000058d15e176280000"}', 12259210, '0xe79596c28c82ac1812cbca86063f1cf9286b321154eae196ee89670a0e70b6cf', 6, 22, '2026-10-10 09:41:54', true),
	('12259258-3', '80', 'DecisionStarted', '{"index":80,"track":1,"proposal":{"lookup":{"hash":"0xf063b48b66892b3a94bc6c83889470e461a69483fc2412b7fdad8778795eef86","len":48}},"tally":{"ayes":"0x0000000000000012f20e38064cc47ca6","nays":0,"support":"0x0000000000000012f20e38064cc47ca6"}}', 12259258, '0xc26f58a365376ebf00132681e301da7e68796ab5efde892178b640e4a1d331d9', NULL, 3, '2026-10-10 09:46:42', true),
	('12259258-4', '80', 'ConfirmStarted', '{"index":80}', 12259258, '0xc26f58a365376ebf00132681e301da7e68796ab5efde892178b640e4a1d331d9', NULL, 4, '2026-10-10 09:46:42', true),
	('12259858-6', '80', 'Confirmed', '{"index":80,"tally":{"ayes":"0x0000000000000012f20e38064cc47ca6","nays":0,"support":"0x0000000000000012f20e38064cc47ca6"}}', 12259858, '0xc25ea9702992135f47b030d7e61225f520d72bd4f057fe84818993776b7d4e00', NULL, 6, '2026-10-10 10:48:12', true),
	('12259888-66', '80', 'Executed', '{"result":{"ok":null}}', 12259888, '0x9745d29b8c5518bfcd937aa6ecee52cb4b47d8536641dabca8c12203acd03bca', NULL, 66, '2026-10-10 10:51:12', true),
	('12300863-12', '80', 'DecisionDepositRefunded', '{"index":80,"who":"1333zsMafds2sKAr8nG3zwXTCHPYv2Nm6CRgakpu6YVGt7nM","amount":"0x0000000000000000058d15e176280000"}', 12300863, '0x3a3da83e7260245376b1dc3a7abeef0318fea8265ba096b52c6ad4327dcc6468', 3, 12, '2026-10-13 08:45:18', false);
INSERT INTO governance.governance_tally_snapshots VALUES
	('79-12249468', '79', 0, 0, 0, 610384174578328293547, 12249468, '2026-10-09 17:08:36'),
	('79-12249517', '79', 0, 0, 0, 610384288871194478231, 12249517, '2026-10-09 17:13:54'),
	('79-12249566', '79', 200000000000000000000, 0, 200000000000000000000, 610384452387139980423, 12249566, '2026-10-09 17:18:48'),
	('79-12249568', '79', 200000000000000000000, 0, 200000000000000000000, 610384456859953219623, 12249568, '2026-10-09 17:19:00'),
	('79-12249571', '79', 349483333333333343398, 0, 349483333333333343398, 610384463720994745716, 12249571, '2026-10-09 17:19:18'),
	('79-12249572', '79', 349483333333333343398, 0, 349483333333333343398, 610384466016503020316, 12249572, '2026-10-09 17:19:24'),
	('79-12250172', '79', 349483333333333343398, 0, 349483333333333343398, 610386384231879288696, 12250172, '2026-10-09 18:19:48'),
	('80-12259158', '80', 0, 0, 0, 610412842757785089930, 12259158, '2026-10-10 09:36:36'),
	('80-12259210', '80', 0, 0, 0, 610412974837317409656, 12259210, '2026-10-10 09:41:54'),
	('80-12259222', '80', 149483333333333343398, 0, 149483333333333343398, 610413002443959147621, 12259222, '2026-10-10 09:43:06'),
	('80-12259225', '80', 349483333333333343398, 0, 349483333333333343398, 610413009033133060714, 12259225, '2026-10-10 09:43:24'),
	('80-12259258', '80', 349483333333333343398, 0, 349483333333333343398, 610413096544165370430, 12259258, '2026-10-10 09:46:42'),
	('80-12259858', '80', 349483333333333343398, 0, 349483333333333343398, 610414735456266280252, 12259858, '2026-10-10 10:48:12');
INSERT INTO governance.governance_tracks VALUES
	('0', 'root', '{"name":"root\u0000\u0000\u0000\u0000\u0000\u0000\u0000\u0000\u0000\u0000\u0000\u0000\u0000\u0000\u0000\u0000\u0000\u0000\u0000\u0000\u0000","maxDeciding":1,"decisionDeposit":"0x00000000000000000de0b6b3a7640000","preparePeriod":600,"decisionPeriod":100800,"confirmPeriod":7200,"minEnactmentPeriod":100,"minApproval":{"reciprocal":{"factor":999999999,"xOffset":999999999,"yOffset":0}},"minSupport":{"reciprocal":{"factor":44628100,"xOffset":90909092,"yOffset":9090908}}}', 12259158),
	('1', 'whitelisted_caller', '{"name":"whitelisted_caller\u0000\u0000\u0000\u0000\u0000\u0000\u0000","maxDeciding":3,"decisionDeposit":"0x0000000000000000058d15e176280000","preparePeriod":100,"decisionPeriod":14400,"confirmPeriod":600,"minEnactmentPeriod":30,"minApproval":{"reciprocal":{"factor":999999999,"xOffset":999999999,"yOffset":0}},"minSupport":{"reciprocal":{"factor":999999999,"xOffset":1072330188,"yOffset":-432548585}}}', 12259158),
	('2', 'referendum_canceller', '{"name":"referendum_canceller\u0000\u0000\u0000\u0000\u0000","maxDeciding":3,"decisionDeposit":"0x0000000000000000016345785d8a0000","preparePeriod":600,"decisionPeriod":43200,"confirmPeriod":600,"minEnactmentPeriod":30,"minApproval":{"reciprocal":{"factor":375000003,"xOffset":500000002,"yOffset":249999998}},"minSupport":{"reciprocal":{"factor":87500001,"xOffset":166666668,"yOffset":-25000001}}}', 12259158),
	('3', 'referendum_killer', '{"name":"referendum_killer\u0000\u0000\u0000\u0000\u0000\u0000\u0000\u0000","maxDeciding":3,"decisionDeposit":"0x0000000000000000016345785d8a0000","preparePeriod":600,"decisionPeriod":43200,"confirmPeriod":600,"minEnactmentPeriod":100,"minApproval":{"reciprocal":{"factor":375000003,"xOffset":500000002,"yOffset":249999998}},"minSupport":{"reciprocal":{"factor":87500001,"xOffset":166666668,"yOffset":-25000001}}}', 12259158),
	('4', 'treasury_spend', '{"name":"treasury_spend\u0000\u0000\u0000\u0000\u0000\u0000\u0000\u0000\u0000\u0000\u0000","maxDeciding":3,"decisionDeposit":"0x00000000000000000de0b6b3a7640000","preparePeriod":600,"decisionPeriod":100800,"confirmPeriod":1800,"minEnactmentPeriod":100,"minApproval":{"reciprocal":{"factor":999999999,"xOffset":999999999,"yOffset":0}},"minSupport":{"reciprocal":{"factor":44628100,"xOffset":90909092,"yOffset":9090908}}}', 12259158);
INSERT INTO governance.governance_vote_calls VALUES
	('12249566-18-79', '1geqfd5rT9atAcC2yV9xjEBgmug3qKnSwKxqa27Lrxfhg7W', 79, '{"standard":{"vote":"0x81","balance":"0x000000000000000ad78ebc5ac6200000"}}', false, 12249566, '0x87b9def779facc9cf94fa4e5310d5307d217f449fa8c39e8b332d85ac78baf44', 5, 18, '2026-10-09 17:18:48'),
	('12249571-20-79', '12ve5fWBPUnhqZFvZtzg5QVND5dBkdWQfC5boWywnsob5PMH', 79, '{"standard":{"vote":"0x81","balance":"0x00000000000000081a7f7bab86a47ca6"}}', false, 12249571, '0x12bb2b485e26f789d70f6e41f44eaab7830994ec799e2be49d6cb5bc112db26c', 7, 20, '2026-10-09 17:19:18'),
	('12259222-19-80', '12ve5fWBPUnhqZFvZtzg5QVND5dBkdWQfC5boWywnsob5PMH', 80, '{"standard":{"vote":"0x81","balance":"0x00000000000000081a7f7bab86a47ca6"}}', false, 12259222, '0x74946f085040573be5de4c56c35c4f450cea8d0a341214442ef969d976ae4982', 6, 19, '2026-10-10 09:43:06'),
	('12259225-17-80', '1geqfd5rT9atAcC2yV9xjEBgmug3qKnSwKxqa27Lrxfhg7W', 80, '{"standard":{"vote":"0x81","balance":"0x000000000000000ad78ebc5ac6200000"}}', false, 12259225, '0xd54591693e8aae7b12d57abdf82cff1e45b7ccc682a39010e246473609579a96', 4, 17, '2026-10-10 09:43:24');
INSERT INTO governance.governance_votes VALUES
	('79-1geqfd5rT9atAcC2yV9xjEBgmug3qKnSwKxqa27Lrxfhg7W', 79, 1, '1geqfd5rT9atAcC2yV9xjEBgmug3qKnSwKxqa27Lrxfhg7W', 'Standard', true, 1, 200000000000000000000, NULL, NULL, NULL, 0, 0, 12249566),
	('79-12ve5fWBPUnhqZFvZtzg5QVND5dBkdWQfC5boWywnsob5PMH', 79, 1, '12ve5fWBPUnhqZFvZtzg5QVND5dBkdWQfC5boWywnsob5PMH', 'Standard', true, 1, 149483333333333343398, NULL, NULL, NULL, 0, 0, 12249571),
	('80-12ve5fWBPUnhqZFvZtzg5QVND5dBkdWQfC5boWywnsob5PMH', 80, 1, '12ve5fWBPUnhqZFvZtzg5QVND5dBkdWQfC5boWywnsob5PMH', 'Standard', true, 1, 149483333333333343398, NULL, NULL, NULL, 0, 0, 12259222),
	('80-1geqfd5rT9atAcC2yV9xjEBgmug3qKnSwKxqa27Lrxfhg7W', 80, 1, '1geqfd5rT9atAcC2yV9xjEBgmug3qKnSwKxqa27Lrxfhg7W', 'Standard', true, 1, 200000000000000000000, NULL, NULL, NULL, 0, 0, 12259225);
INSERT INTO governance.governance_whitelisted_calls VALUES
	('0x3437c169b25cdf20c03e0241c5204151ffc478c4c8b2878f6fffe27052f5f029', 12249554, NULL, 12250202, '{"ok":{"actualWeight":null,"paysFee":"Yes"}}'),
	('0x774793b5b7b85afd9f10cd6207f363dba366c17357112eb11092253377c4be9c', 12259265, NULL, 12259888, '{"ok":{"actualWeight":{"refTime":10830611064,"proofSize":68016},"paysFee":"Yes"}}');
INSERT INTO governance._metadata VALUES
	('genesisHash', '"0x61ea8a51fd4a058ee8c0e86df0a89cc85b8b67a0a66432893d09719050c9f540"', '2026-10-15 19:18:38.92+00', '2026-10-15 19:18:38.92+00'),
	('chain', '"Hyperbridge (Nexus)"', '2026-10-15 19:18:38.92+00', '2026-10-15 19:18:38.92+00'),
	('specName', '"nexus"', '2026-10-15 19:18:38.92+00', '2026-10-15 19:18:38.92+00'),
	('indexerNodeVersion', '"6.4.9-0"', '2026-10-15 19:18:38.92+00', '2026-10-15 19:18:38.92+00'),
	('startHeight', '12135052', '2026-10-15 19:18:38.92+00', '2026-10-15 19:18:38.92+00'),
	('dynamicDatasources', '[]', '2026-10-15 19:18:38.922+00', '2026-10-15 19:18:38.922+00'),
	('indexerHealthy', 'false', '2026-10-15 19:19:38.759+00', '2026-10-15 23:59:23.912+00'),
	('deployments', '"{\"12135052\":\"/app\"}"', '2026-10-15 19:18:40.856+00', '2026-10-15 19:18:40.856+00'),
	('historicalStateEnabled', 'false', '2026-10-15 19:18:38.92+00', '2026-10-15 19:18:38.92+00'),
	('runnerNode', '"@subql/node"', '2026-10-15 19:18:38.92+00', '2026-10-15 19:18:38.92+00'),
	('runnerNodeVersion', '">=4.0.0"', '2026-10-15 19:18:38.92+00', '2026-10-15 19:18:38.92+00'),
	('runnerQuery', '"@subql/query"', '2026-10-15 19:18:38.92+00', '2026-10-15 19:18:38.92+00'),
	('runnerQueryVersion', '"*"', '2026-10-15 19:18:38.92+00', '2026-10-15 19:18:38.92+00'),
	('schemaMigrationCount', '2', '2026-10-15 19:18:38.910504+00', '2026-10-15 20:48:21.424626+00'),
	('targetHeight', '12337798', '2026-10-15 19:18:51.311+00', '2026-10-15 19:18:51.311+00'),
	('processedBlockCount', '202747', '2026-10-15 19:18:38.910504+00', '2026-10-15 23:58:13.052751+00'),
	('lastProcessedHeight', '12337798', '2026-10-15 19:18:51.311+00', '2026-10-15 19:18:51.311+00'),
	('lastProcessedTimestamp', '1792108693051', '2026-10-15 19:18:51.311+00', '2026-10-15 19:18:51.311+00'),
	('lastProcessedBlockTimestamp', '1792108656000', '2026-10-15 19:18:51.311+00', '2026-10-15 19:18:51.311+00');
