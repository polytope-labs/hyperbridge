-- CreateTable
CREATE TABLE "OutboundRotationClaims" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "dest" TEXT NOT NULL,
    "set_id" BIGINT NOT NULL,
    "rotation_height" BIGINT NOT NULL,
    "status" TEXT NOT NULL,
    "created_at" INTEGER NOT NULL,
    "updated_at" INTEGER NOT NULL,
    "note" TEXT
);

-- CreateIndex
CREATE INDEX "OutboundRotationClaims_status_idx" ON "OutboundRotationClaims"("status");

-- CreateIndex
CREATE UNIQUE INDEX "OutboundRotationClaims_dest_set_id_key" ON "OutboundRotationClaims"("dest", "set_id");
