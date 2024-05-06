-- CreateTable
CREATE TABLE "Deliveries" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "hash" TEXT NOT NULL,
    "source_chain" TEXT NOT NULL,
    "dest_chain" TEXT NOT NULL,
    "delivery_type" INTEGER NOT NULL,
    "created_at" INTEGER NOT NULL,
    "height" INTEGER NOT NULL
);
