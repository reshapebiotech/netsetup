declare global {
  namespace App {
    interface PageState {
      deviceAddress?: string;
      setupWifiName?: string;
    }
    interface PageData {
      baseUrl: string;
    }
  }
}

export {};
