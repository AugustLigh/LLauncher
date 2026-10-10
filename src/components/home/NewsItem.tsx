import { openUrl } from "@tauri-apps/plugin-opener";
import { Announcement } from "../../bindings";
import Icon from "../common/Icon";
import "./NewsItem.css";

export interface NewsItemProps {
  item: Announcement;
}

export default function NewsItem({ item }: NewsItemProps) {
  const Tag = item.jump_url ? "a" : "div";
  return (
    <Tag
      href={item.jump_url || undefined}
      title={item.content}
      className="news-item"
      onClick={
        item.jump_url
          ? (e) => {
              e.preventDefault();
              openUrl(item.jump_url!).catch(console.error);
            }
          : undefined
      }
    >
      <span>{item.content}</span>
      {item.jump_url && <Icon name="external" size={14} />}
    </Tag>
  );
}
