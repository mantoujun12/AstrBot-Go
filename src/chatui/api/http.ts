// Ported from AstrBot dashboard (MIT, CodedThemes); see src/chatui/LICENSE.
// Island glue: upstream axios instance with the JWT interceptors dropped.
// The custom adapter routes every request through Tauri commands; the API key
// stays in Rust, so headers are never attached or logged here.

import axios from "axios";
import { chatuiAdapter } from "./adapter";

export const apiV1Client = axios.create({
  baseURL: "/api/v1",
  adapter: chatuiAdapter,
});

/** Plain fetch for the (stubbed) streaming helper paths that never run. */
export const fetchWithAuth: typeof window.fetch = (input, init) =>
  window.fetch(input, init);
