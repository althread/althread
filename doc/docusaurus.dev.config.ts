import config from './docusaurus.config';
import type { Config } from "@docusaurus/types";

const devConfig: Config = JSON.parse(JSON.stringify(config));

devConfig.baseUrl = '/dev/';

if (devConfig.presets?.[0]?.[1]?.docs?.editUrl) {
  devConfig.presets[0][1].docs.editUrl = "https://github.com/althread/althread/tree/dev/doc";
}


export default devConfig;