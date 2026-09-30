// System Store - App-level state
import { isBrowser as browser } from '$lib/utils/env';
import type { ModelDone, ModelProgress } from '$lib/api/events';

export type ModelDownloadState = 'idle' | 'downloading' | 'ready' | 'failed';

function createSystemStore() {
    let isOnline = $state(true);
    let notificationPermission = $state<NotificationPermission>('default');
    let appVersion = $state('1.0.0');
    let updateAvailable = $state(false);
    let modelState = $state<ModelDownloadState>('idle');
    let modelProgress = $state<ModelProgress | null>(null);
    let modelError = $state<string | null>(null);

    function setOnline(v: boolean) { isOnline = v; }
    function setNotificationPermission(p: NotificationPermission) { notificationPermission = p; }
    function setAppVersion(v: string) { appVersion = v; }
    function setUpdateAvailable(v: boolean) { updateAvailable = v; }

    function setModelProgress(p: ModelProgress) {
        modelState = 'downloading';
        modelProgress = p;
    }
    function setModelDone(d: ModelDone) {
        if (d.ok) {
            modelState = 'ready';
            modelProgress = null;
        } else {
            modelState = 'failed';
            modelError = d.error || '模型下载失败';
        }
    }
    /** Dismiss a failure notice; the next kickoff retries the download. */
    function dismissModelError() {
        if (modelState === 'failed') modelState = 'idle';
        modelError = null;
    }

    if (browser && 'Notification' in window) {
        notificationPermission = Notification.permission;
    }

    return { get isOnline() { return isOnline; }, get notificationPermission() { return notificationPermission; }, get appVersion() { return appVersion; }, get updateAvailable() { return updateAvailable; }, get modelState() { return modelState; }, get modelProgress() { return modelProgress; }, get modelError() { return modelError; }, setOnline, setNotificationPermission, setAppVersion, setUpdateAvailable, setModelProgress, setModelDone, dismissModelError };
}

export const systemStore = createSystemStore();
