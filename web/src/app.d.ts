declare global {
  namespace App {
    interface PageState {
      deviceAddress?: string;
    }
    interface PageData {
      baseUrl: string;
    }
  }
}

export {};
