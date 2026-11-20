-- AlterTable
ALTER TABLE "UnprofitableMessages" ADD COLUMN "attempts" INTEGER NOT NULL DEFAULT 0;
ALTER TABLE "UnprofitableMessages" ADD COLUMN "created_at" INTEGER NOT NULL DEFAULT 0;
ALTER TABLE "UnprofitableMessages" ADD COLUMN "next_attempt_at" INTEGER NOT NULL DEFAULT 0;
