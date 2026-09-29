// System Store - App-level state
import { browser } from '$app/environment';

function createSystemStore() {
    let isOnline = $state(true);
    let notificationPermission = $state<NotificationPermission>('default');
    let appVersion = $state('1.0.0');
    let updateAvailable = $state(false);
    
    function setOnline(v: boolean) { isOnline = v; }
    function setNotificationPermission(p: NotificationPermission) { notificationPermission = p; }
    function setAppVersion(v: string) { appVersion = v; }
    function setUpdateAvailable(v: boolean) { updateAvailable = v; }
    
    if (browser && 'Notification' in window) {
        notificationPermission = Notification.permission;
    }
    
    return { get isOnline() { return isOnline; }, get notificationPermission() { return notificationPermission; }, get appVersion() { return appVersion; }, get updateAvailable() { return updateAvailable; }, setOnline, setNotificationPermission, setAppVersion, setUpdateAvailable };
}

export const systemStore = createSystemStore();