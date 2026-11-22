export interface ExtensionEntry {
  id: string;
  icon: string;
  /** Route the card opens. */
  to: string;
}

export const EXTENSIONS: ExtensionEntry[] = [
  { id: "plugins", icon: "mdi-puzzle-outline", to: "/extensions/plugins" },
  { id: "skills", icon: "mdi-school-outline", to: "/extensions/skills" },
];
