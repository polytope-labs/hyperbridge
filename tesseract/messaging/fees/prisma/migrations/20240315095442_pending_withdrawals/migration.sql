-- CreateTable
CREATE TABLE "PendingWithdrawal" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "dest" TEXT NOT NULL,
    "encoded" BLOB NOT NULL
);
