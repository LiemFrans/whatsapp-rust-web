import type { ReactNode } from "react";
import { Icon } from "./Icons";
import "./Modal.css";

interface ModalProps {
  title: string;
  open: boolean;
  onClose: () => void;
  children: ReactNode;
}

export function Modal({ title, open, onClose, children }: ModalProps) {
  if (!open) return null;

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h3>{title}</h3>
          <button className="modal-close" onClick={onClose}>
            <Icon.Close />
          </button>
        </div>
        <div className="modal-body">{children}</div>
      </div>
    </div>
  );
}
