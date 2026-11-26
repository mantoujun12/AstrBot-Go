// Ported from AstrBot dashboard (MIT, CodedThemes); see src/chatui/LICENSE.
import { ref, computed, reactive } from 'vue';
import { chatApi } from '@/api/v1';

export interface Session {
    session_id: string;
    display_name: string | null;
    updated_at: string;
    platform_id: string;
    creator: string;
    is_group: number;
    created_at: string;
}

export function useSessions(chatboxMode: boolean = false) {
    void chatboxMode;
    const sessions = ref<Session[]>([]);
    const sessionsPagination = reactive({
        page: 0,
        hasMore: false,
        loading: false,
        error: false,
        append: false,
    });
    let sessionsRequestId = 0;
    const selectedSessions = ref<string[]>([]);
    const currSessionId = ref('');
    const pendingSessionId = ref<string | null>(null);
    // 编辑标题相关
    const editTitleDialog = ref(false);
    const editingTitle = ref('');
    const editingSessionId = ref('');

    const getCurrentSession = computed(() => {
        if (!currSessionId.value) return null;
        return sessions.value.find(s => s.session_id === currSessionId.value);
    });

    async function getSessions(append = false) {
        if (append && (sessionsPagination.loading || !sessionsPagination.hasMore)) return;
        const requestId = ++sessionsRequestId;
        const lastPage = append ? sessionsPagination.page + 1 : Math.max(1, sessionsPagination.page);
        const loaded = append ? [...sessions.value] : [];
        sessionsPagination.loading = true;
        sessionsPagination.error = false;
        sessionsPagination.append = append;
        try {
            // Refresh the loaded range after mutations so older visible sessions stay accessible.
            for (let page = append ? lastPage : 1; page <= lastPage; page++) {
                const response = await chatApi.listSessions({ page, page_size: 30 });
                if (requestId !== sessionsRequestId) return;
                if (response.data.status !== 'ok') {
                    throw new Error(response.data.message || 'Failed to load sessions');
                }
                const payload = response.data.data;
                loaded.push(...payload.sessions);
                const hasMore = page * payload.page_size < payload.total;
                if (page === lastPage || !hasMore) {
                    sessions.value = [...new Map(loaded.map(session => [session.session_id, session])).values()];
                    sessionsPagination.page = page;
                    sessionsPagination.hasMore = hasMore;
                    break;
                }
            }
        } catch (err: any) {
            if (requestId !== sessionsRequestId) return;
            sessionsPagination.error = true;
            console.error(err);
        } finally {
            if (requestId === sessionsRequestId) sessionsPagination.loading = false;
        }
    }

    async function deleteSession(sessionId: string) {
        try {
            await chatApi.deleteSession(sessionId);
            await getSessions();
            currSessionId.value = '';
            selectedSessions.value = [];
        } catch (err) {
            console.error(err);
        }
    }

    function showEditTitleDialog(sessionId: string, title: string) {
        editingSessionId.value = sessionId;
        editingTitle.value = title || '';
        editTitleDialog.value = true;
    }

    async function saveTitle() {
        if (!editingSessionId.value) return;

        const trimmedTitle = editingTitle.value.trim();
        try {
            await chatApi.updateSession(editingSessionId.value, {
                display_name: trimmedTitle,
            });

            // 更新本地会话标题
            const session = sessions.value.find(s => s.session_id === editingSessionId.value);
            if (session) {
                session.display_name = trimmedTitle;
            }
            editTitleDialog.value = false;
        } catch (err) {
            console.error('重命名会话失败：', err);
        }
    }

    function updateSessionTitle(sessionId: string, title: string) {
        const session = sessions.value.find(s => s.session_id === sessionId);
        if (session) {
            session.display_name = title;
        }
    }

    function newChat(closeMobileSidebar?: () => void) {
        currSessionId.value = '';
        selectedSessions.value = [];

        if (closeMobileSidebar) {
            closeMobileSidebar();
        }
    }

    return {
        sessions,
        sessionsPagination,
        selectedSessions,
        currSessionId,
        pendingSessionId,
        editTitleDialog,
        editingTitle,
        editingSessionId,
        getCurrentSession,
        getSessions,
        deleteSession,
        showEditTitleDialog,
        saveTitle,
        updateSessionTitle,
        newChat
    };
}
