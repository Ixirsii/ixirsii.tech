"use client";

import { useRouter } from "next/navigation";
import { type ComponentRef, type ReactNode, useEffect, useRef } from "react";
import { createPortal } from "react-dom";

export function Modal({
  children,
}: Readonly<{ children: ReactNode }>): ReactNode {
  const router = useRouter();
  const dialogRef = useRef<ComponentRef<"dialog">>(null);

  useEffect(() => {
    if (!dialogRef.current?.open) {
      dialogRef.current?.showModal();
    }
  }, []);

  function onDismiss() {
    router.back();
  }

  return createPortal(
    <div className="modal-backdrop">
      <dialog ref={dialogRef} className="modal" onClose={onDismiss}>
        <div className="overflow-y-scroll">
          {children}
          <button onClick={onDismiss} className="close-button" />
        </div>
      </dialog>
    </div>,
    document.getElementById("modal-root")!,
  );
}
