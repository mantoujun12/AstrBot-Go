export interface ExtensionEntry {
  id: string;
  icon: string;
}

// shortcut: fake placeholder entries, replace before shipping real extensions
export const EXTENSIONS: ExtensionEntry[] = [
  { id: "hue", icon: "mdi-palette-outline" },
  { id: "palette", icon: "mdi-watermark" },
  { id: "converter", icon: "mdi-swap-horizontal" },
  { id: "playground", icon: "mdi-code-braces" },
];
