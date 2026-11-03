import type { NetworkSourceKind } from "../services/network";
import type { AppConfig } from "../types/launcher";

export type NetworkSourceKey = NetworkSourceKind;

export interface NetworkSourcePreset {
  value: string;
  labelKey?: string;
  label?: string;
}

export interface NetworkSourceDefinition {
  key: NetworkSourceKey;
  configField: keyof Pick<
    AppConfig,
    | "github_proxy"
    | "pypi_mirror"
    | "nodejs_mirror"
    | "npm_registry"
    | "python_mirror"
    | "uv_mirror"
  >;
  titleKey: string;
  defaultLabelKey: string;
  defaultUrl: string;
  presets: NetworkSourcePreset[];
}

export const NETWORK_SOURCES: NetworkSourceDefinition[] = [
  {
    key: "github",
    configField: "github_proxy",
    titleKey: "settings.network.githubProxy",
    defaultLabelKey: "settings.network.sources.default",
    defaultUrl: "https://github.com",
    presets: [
      { value: "https://gh-proxy.com/", label: "gh-proxy.com" },
      { value: "https://gh-proxy.org/", label: "gh-proxy.org" },
      { value: "https://hk.gh-proxy.com/", label: "hk.gh-proxy.com" },
      { value: "https://edgeone.gh-proxy.com/", label: "edgeone.gh-proxy.com" },
      { value: "https://gh.llkk.cc/", label: "gh.llkk.cc" },
    ],
  },
  {
    key: "pypi",
    configField: "pypi_mirror",
    titleKey: "settings.network.pypiMirror",
    defaultLabelKey: "settings.network.sources.default",
    defaultUrl: "https://pypi.org/simple",
    presets: [
      {
        value: "https://mirrors.aliyun.com/pypi/simple",
        labelKey: "settings.network.sources.aliyun",
      },
      {
        value: "https://mirrors.cloud.tencent.com/pypi/simple",
        labelKey: "settings.network.sources.tencent",
      },
      {
        value: "https://pypi.mirrors.ustc.edu.cn/simple",
        labelKey: "settings.network.sources.ustc",
      },
      {
        value: "https://pypi.tuna.tsinghua.edu.cn/simple",
        labelKey: "settings.network.sources.tuna",
      },
      {
        value: "https://mirrors.cernet.edu.cn/pypi/web/simple",
        labelKey: "settings.network.sources.cernet",
      },
    ],
  },
  {
    key: "nodejs",
    configField: "nodejs_mirror",
    titleKey: "settings.network.nodejsMirror",
    defaultLabelKey: "settings.network.sources.default",
    defaultUrl: "https://nodejs.org/dist",
    presets: [
      {
        value: "https://npmmirror.com/mirrors/node/",
        labelKey: "settings.network.sources.npmmirror",
      },
      {
        value: "https://mirrors.tuna.tsinghua.edu.cn/nodejs-release/",
        labelKey: "settings.network.sources.tuna",
      },
      {
        value: "https://mirrors.nju.edu.cn/nodejs-release/",
        labelKey: "settings.network.sources.nju",
      },
      {
        value: "https://mirrors.cernet.edu.cn/nodejs-release/",
        labelKey: "settings.network.sources.cernet",
      },
    ],
  },
  {
    key: "npm",
    configField: "npm_registry",
    titleKey: "settings.network.npmRegistry",
    defaultLabelKey: "settings.network.sources.default",
    defaultUrl: "https://registry.npmjs.org",
    presets: [
      {
        value: "https://registry.npmmirror.com/",
        labelKey: "settings.network.sources.npmmirror",
      },
      {
        value: "https://mirrors.cloud.tencent.com/npm/",
        labelKey: "settings.network.sources.tencent",
      },
      {
        value: "https://repo.huaweicloud.com/repository/npm/",
        labelKey: "settings.network.sources.huawei",
      },
    ],
  },
  {
    key: "python",
    configField: "python_mirror",
    titleKey: "settings.network.pythonMirror",
    defaultLabelKey: "settings.network.sources.followGithub",
    defaultUrl: "https://github.com/astral-sh/python-build-standalone",
    presets: [
      {
        value:
          "https://mirrors.ustc.edu.cn/github-release/astral-sh/python-build-standalone/LatestRelease/",
        labelKey: "settings.network.sources.ustc",
      },
      {
        value:
          "https://mirrors.cernet.edu.cn/github-release/astral-sh/python-build-standalone/LatestRelease/",
        labelKey: "settings.network.sources.cernet",
      },
    ],
  },
  {
    key: "uv",
    configField: "uv_mirror",
    titleKey: "settings.network.uvMirror",
    defaultLabelKey: "settings.network.sources.followGithub",
    defaultUrl: "https://github.com/astral-sh/uv",
    presets: [
      {
        value:
          "https://mirrors.ustc.edu.cn/github-release/astral-sh/uv/LatestRelease/",
        labelKey: "settings.network.sources.ustc",
      },
      {
        value:
          "https://mirrors.cernet.edu.cn/github-release/astral-sh/uv/LatestRelease/",
        labelKey: "settings.network.sources.cernet",
      },
    ],
  },
];
