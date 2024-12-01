import type * as Preset from "@docusaurus/preset-classic";
import type { Config } from "@docusaurus/types";
import { themes as prismThemes } from "prism-react-renderer";

const config: Config = {
  title: "Althread",
  tagline: "Documentation officielle du langage de programmation Althread",
  favicon: "img/favicon.ico",

  url: "https://althread.github.io",
  baseUrl: "/",

  organizationName: "althread",
  projectName: "althread",
  deploymentBranch: "main",

  onBrokenLinks: "ignore",
  onBrokenMarkdownLinks: "warn",

  i18n: {
    defaultLocale: "fr",
    locales: ["fr"],
  },

  presets: [
    [
      "classic",
      {
        docs: {
          sidebarPath: "./sidebars.ts",
          editUrl: "https://github.com/althread/althread/tree/main/doc",
        },
        theme: {
          customCss: "./src/css/custom.css",
        },
      } satisfies Preset.Options,
    ],
  ],

  themeConfig: {
    navbar: {
      title: "Althread",
      logo: {
        alt: "Althread Logo",
        src: "img/logo.svg",
      },
      items: [
        {
          type: "docSidebar",
          sidebarId: "guideSidebar",
          position: "left",
          label: "Guide",
        },
        {
          type: "docSidebar",
          sidebarId: "apiSidebar",
          position: "left",
          label: "Références",
        },
        {
          type: "docSidebar",
          sidebarId: "exampleSidebar",
          position: "left",
          label: "Exemples",
        },
        {
          href: "https://github.com/althread/althread/",
          label: "GitHub",
          position: "right",
        },
/*         {
          type: "localeDropdown",
          position: "right",
        }, */
      ],
    },
    footer: {
      style: "dark",
      links: [
        {
          title: "Documentation",
          items: [
            {
              label: "Guide",
              to: "/docs/guide/intro",
            },
            {
              label: "Références",
              to: "/docs/api",
            },
            {
              label: "Exemples",
              to: "/docs/examples",
            },
          ],
        },

        {
          title: "Plus",
          items: [
            {
              label: "GitHub",
              href: "https://github.com/althread/althread/",
            },
            {
              label: "Editeur",
              href: "../editor/",
            }
          ],
        },
      ],
      copyright: `Copyright © ${new Date().getFullYear()} Romain Bourdain.`,
    },
    prism: {
      theme: prismThemes.github,
      darkTheme: prismThemes.dracula,
    },
  } satisfies Preset.ThemeConfig,
};

export default config;
