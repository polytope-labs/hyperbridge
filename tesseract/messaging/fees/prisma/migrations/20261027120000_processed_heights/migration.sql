-- CreateTable
CREATE TABLE "ProcessedHeights" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "source" TEXT NOT NULL,
    "dest" TEXT NOT NULL,
    "height" BIGINT NOT NULL,
    "updated_at" INTEGER NOT NULL
);

-- CreateIndex
CREATE UNIQUE INDEX "ProcessedHeights_source_dest_key" ON "ProcessedHeights"("source", "dest");
