/** @type {import('next-sitemap').IConfig} */
module.exports = {
    siteUrl:
        process.env.NEXT_PUBLIC_SITE_URL || "https://docs.hyperbridge.network",
    generateRobotsTxt: true,
    generateIndexSitemap: false,
    outDir: "./out",
    exclude: ["/api/*", "/og/*", "/llms-full.txt/*"],
    robotsTxtOptions: {
        policies: [
            {
                userAgent: "*",
                allow: "/",
            },
        ],
    },
};
