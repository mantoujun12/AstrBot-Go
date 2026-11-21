// Ported from AstrBot dashboard (MIT, CodedThemes); see src/chatui/LICENSE.
// v1 stub of composables/useMediaHandling.ts: attachments are disabled, so
// every staged-file view stays empty and every handler is a no-op that keeps
// the upstream call sites compiling unchanged.

import { ref } from "vue";

export interface FailedUploadView {
  name: string;
  error: string;
}

export interface ActiveUploadView {
  name: string;
  percent: number;
}

export interface StagedFileInfo {
  attachment_id: string;
  filename: string;
  original_name: string;
  url: string;
  type: string;
}

export function useMediaHandling() {
  const stagedFiles = ref<StagedFileInfo[]>([]);
  const stagedImagesUrl = ref<string[]>([]);
  const stagedAudioUrl = ref("");
  const stagedNonImageFiles = ref<StagedFileInfo[]>([]);
  const failedUploadViews = ref<FailedUploadView[]>([]);
  const activeUploadViews = ref<ActiveUploadView[]>([]);

  return {
    stagedFiles,
    stagedImagesUrl,
    stagedAudioUrl,
    stagedNonImageFiles,
    failedUploadViews,
    activeUploadViews,
    processAndUploadImage: async (_file: File) => {},
    processAndUploadFile: async (_file: File) => {},
    handlePaste: () => {},
    removeImage: () => {},
    removeAudio: () => {},
    removeFile: () => {},
    retryFailedUpload: () => {},
    discardFailedUpload: () => {},
    cancelActiveUpload: () => {},
    clearStaged: (_options?: { revokeUrls?: boolean }) => {},
    cleanupMediaCache: () => {},
  };
}
