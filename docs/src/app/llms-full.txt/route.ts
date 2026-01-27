import { backendSource, getLLMText, protocolSource } from "@/lib/source";

export const revalidate = false;

export async function GET() {
    const scan = backendSource
        .getPages()
        .concat(protocolSource.getPages())
        .map(getLLMText);

    const scanned = await Promise.all(scan);

    return new Response(scanned.join("\n\n"));
}
