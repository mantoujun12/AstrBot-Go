export interface ExtensionEntry {
  id: string;
  icon: string;
  title: string;
  description: string;
}

// shortcut: fake placeholder data, replace before shipping real extensions
export const EXTENSIONS: ExtensionEntry[] = [
  {
    id: "hue",
    icon: "mdi-palette-outline",
    title: "Hue",
    description: "Material You color picker",
  },
  {
    id: "palette",
    icon: "mdi-watermark",
    title: "Palette",
    description: "Curated color palettes",
  },
  {
    id: "converter",
    icon: "mdi-swap-horizontal",
    title: "Converter",
    description: "Format conversion tools",
  },
  {
    id: "playground",
    icon: "mdi-code-braces",
    title: "Playground",
    description: "Quick snippet sandbox",
  },
];
