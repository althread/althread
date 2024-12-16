import type * as Preset from "@docusaurus/preset-classic";
import type { Config } from "@docusaurus/types";
import { themes as prismThemes } from "prism-react-renderer";


const customPrismTheme = prismThemes.github;
customPrismTheme.styles.push({
  types: ["system-block"],
  style: {
    color: "#b830ff",
    fontWeight: "bold",
  },
});
customPrismTheme.styles.push({
  types: ["constant"],
  style: {
    color: "rgb(235 40 50)",
    fontWeight: "normal",
  },
});
customPrismTheme.styles.push({
  types: ["variable"],
  style: {
    color: "rgb(68 70 70)",
  },
});
customPrismTheme.styles.push({
  types: ["function"],
  style: {
    color: "rgb(50, 147, 23)",
  },
});

const customPrismThemeDark = prismThemes.dracula;
customPrismThemeDark.styles.push({
  types: ["system-block"],
  style: {
    color: "#b830ff",
    fontWeight: "bold",
  },
});
customPrismThemeDark.styles.push({
  types: ["constant"],
  style: {
    color: "rgb(255 164 169)",
    fontWeight: "normal",
  },
});
customPrismThemeDark.styles.push({
  types: ["variable"],
  style: {
    color: "rgb(197 197 197)",
  },
});
customPrismThemeDark.styles.push({
  types: ["bool", "int", "float", "number"],
  style: {
    color: "rgb(225 225 175)",
  },
});


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
    defaultLocale: 'fr',
    locales: ['en', 'fr'],
    localeConfigs: {
      en: {
        htmlLang: 'en-GB',
      },
      fr: {
        htmlLang: 'fr-FR',
      },
    },
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
          type: 'localeDropdown',
          position: 'right',
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
      theme: customPrismTheme,
      darkTheme: customPrismThemeDark,
      magicComments: [
        // Remember to extend the default highlight class name as well!
        {
          className: 'theme-code-block-highlighted-line',
          line: 'highlight-next-line',
          block: {start: 'highlight-start', end: 'highlight-end'},
        },
        {
          className: 'code-block-error-line',
          line: 'This will error',
        },
      ],
    },
  } satisfies Preset.ThemeConfig,
};

export default config;
