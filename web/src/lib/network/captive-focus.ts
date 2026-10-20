export function isAppleCaptiveBrowser(userAgent: string): boolean {
  return /CaptiveNetworkSupport/i.test(userAgent)
    || (/iPhone|iPad|iPod/.test(userAgent) && /AppleWebKit/.test(userAgent)
      && !/Safari\/|CriOS\/|FxiOS\/|EdgiOS\//.test(userAgent));
}

// Keep background focus changes in Apple's captive browser from opening the keyboard.
// Require a user interaction before entering a field, without polling for focus.
export function captiveFocus(element: HTMLElement): (() => void) | undefined {
  if (!isAppleCaptiveBrowser(navigator.userAgent)) return;

  let intended: HTMLInputElement | null = null;
  let keyboardUntil = 0;
  let pointerUntil = 0;

  function inputFor(target: EventTarget | null): HTMLInputElement | null {
    if (!(target instanceof Element)) return null;
    const control = target instanceof HTMLInputElement ? target : target.closest('label')?.control;
    return control instanceof HTMLInputElement && element.contains(control)
      && ['text', 'password', 'email', 'number', 'search', 'tel', 'url'].includes(control.type)
      ? control : null;
  }

  function pointerDown(event: PointerEvent) {
    if (!event.isTrusted) return;
    intended = inputFor(event.target);
    pointerUntil = performance.now() + 1000;
    keyboardUntil = 0;
    const active = document.activeElement;
    const button = event.target instanceof Element ? event.target.closest('button') : null;
    if (!intended && active instanceof HTMLInputElement && element.contains(active)
      && (!active.id || button?.getAttribute('aria-controls') !== active.id)) active.blur();
  }

  function keyDown(event: KeyboardEvent) {
    if (event.isTrusted && event.key === 'Tab') keyboardUntil = performance.now() + 1000;
  }

  function click(event: MouseEvent) {
    const input = inputFor(event.target);
    // Keyboard and assistive technology activation can arrive without a pointer event.
    if (event.isTrusted && event.detail === 0 && input) {
      intended = input;
      pointerUntil = performance.now() + 1000;
      input.focus();
    }
  }

  function focusIn(event: FocusEvent) {
    const input = inputFor(event.target);
    if (!input) { keyboardUntil = 0; return; }
    const allowed = (intended === input && performance.now() <= pointerUntil)
      || performance.now() <= keyboardUntil;
    intended = null;
    keyboardUntil = 0;
    if (!allowed) input.blur();
  }

  function focusOut(event: FocusEvent) {
    if (!inputFor(event.target)) return;
    // Preserve the keyboard's Next/Previous controls while the user is editing.
    intended = inputFor(event.relatedTarget);
    pointerUntil = performance.now() + 1000;
  }

  document.addEventListener('pointerdown', pointerDown, true);
  document.addEventListener('keydown', keyDown, true);
  document.addEventListener('click', click, true);
  document.addEventListener('focusin', focusIn, true);
  element.addEventListener('focusout', focusOut, true);
  return () => {
    document.removeEventListener('pointerdown', pointerDown, true);
    document.removeEventListener('keydown', keyDown, true);
    document.removeEventListener('click', click, true);
    document.removeEventListener('focusin', focusIn, true);
    element.removeEventListener('focusout', focusOut, true);
  };
}
