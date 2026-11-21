// Ported from AstrBot dashboard (MIT, CodedThemes); see src/chatui/LICENSE.
import common from "./common.json";
import navigation from "./navigation.json";
import shared from "./shared.json";
import chat from "./chat.json";
import config from "./config.json";
import provider from "./provider.json";

export default {
  core: { common, navigation, shared },
  features: { chat, config, provider },
};