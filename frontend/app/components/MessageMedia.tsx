// Rich document / media preview component for chat message bubbles.

import type { ApiMessage } from "../lib/types";
import { clsx, fileExtension, documentMeta } from "../lib/helpers";

export function MessageMedia({ message }: { message: ApiMessage }) {
  if (!message.media?.download_path) return null;

  const media = message.media;
  const downloadPath = media.download_path!;

  if (media.kind === "image" || media.kind === "sticker") {
    return (
      <img
        src={downloadPath}
        alt={media.caption ?? media.file_name ?? media.kind}
        className={clsx(
          "mb-2 max-h-72 rounded-2xl object-contain",
          media.kind === "sticker" && "max-h-40 bg-transparent",
        )}
      />
    );
  }

  if (media.kind === "video") {
    return (
      <video
        src={downloadPath}
        controls
        playsInline
        className="mb-2 max-h-80 rounded-2xl bg-black"
      />
    );
  }

  if (media.kind === "audio") {
    return <audio src={downloadPath} controls className="mb-2 w-full max-w-xs" />;
  }

  const fileName = media.file_name ?? media.title ?? "Document";
  const extension = fileExtension(media.file_name, media.mime_type);
  const meta = documentMeta(media);

  return (
    <a
      href={downloadPath}
      target="_blank"
      rel="noreferrer"
      className="mb-2 block w-[320px] max-w-full overflow-hidden rounded-2xl bg-[#f0f2f5] text-sm text-wa-text shadow-sm ring-1 ring-black/5 transition hover:bg-[#e8ecef]"
    >
      <div className="relative h-40 bg-[#dfe5e7]">
        {media.preview_image_url ? (
          <img
            src={media.preview_image_url}
            alt={fileName}
            className="h-full w-full object-cover"
          />
        ) : (
          <div className="flex h-full items-center justify-center bg-gradient-to-br from-[#dde4e6] to-[#c9d3d8] px-4 text-center">
            <div>
              <div className="text-4xl font-semibold tracking-tight text-[#41525d]">{extension}</div>
              <div className="mt-2 text-xs font-medium uppercase tracking-[0.2em] text-[#667781]">
                {media.page_count ? `${media.page_count} page${media.page_count > 1 ? "s" : ""}` : "Document preview"}
              </div>
            </div>
          </div>
        )}
        <div className="absolute left-3 top-3 rounded-lg bg-white/90 px-2 py-1 text-[11px] font-semibold uppercase tracking-wide text-[#111b21] shadow-sm">
          {extension}
        </div>
        <div className="absolute inset-x-0 bottom-0 bg-gradient-to-t from-black/60 via-black/20 to-transparent px-3 py-3">
          <div className="truncate text-sm font-medium text-white">{fileName}</div>
          {media.title && media.title !== fileName ? (
            <div className="truncate text-xs text-white/80">{media.title}</div>
          ) : null}
        </div>
      </div>

      <div className="flex items-center gap-3 px-3 py-3">
        <div className="flex h-11 w-11 shrink-0 items-center justify-center rounded-xl bg-white text-xs font-semibold uppercase tracking-wide text-[#54656f] shadow-sm">
          {extension}
        </div>
        <div className="min-w-0 flex-1">
          <div className="truncate font-medium text-wa-text">{fileName}</div>
          <div className="truncate text-xs text-wa-text-secondary">{meta || "Tap to open"}</div>
        </div>
        <div className="shrink-0 text-xs font-semibold text-wa-teal">Open</div>
      </div>
    </a>
  );
}
