-- CreateTable
CREATE TABLE "OutboundRequestClaims" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "dest" TEXT NOT NULL,
    "commitment" TEXT NOT NULL,
    "encoded_request" BLOB NOT NULL,
    "delivery_height" BIGINT NOT NULL,
    "status" TEXT NOT NULL,
    "created_at" INTEGER NOT NULL,
    "updated_at" INTEGER NOT NULL,
    "note" TEXT
);

-- CreateIndex
CREATE INDEX "OutboundRequestClaims_status_idx" ON "OutboundRequestClaims"("status");

-- CreateIndex
CREATE UNIQUE INDEX "OutboundRequestClaims_commitment_key" ON "OutboundRequestClaims"("commitment");
