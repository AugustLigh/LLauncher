import { useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useTranslation } from "../../i18n";
import Icon from "../common/Icon";
import "./BannerCarousel.css";

export default function BannerCarousel({ banners = [] }) {
  const { t } = useTranslation();
  const [active, setActive] = useState(0);
  const [failed, setFailed] = useState([]);
  const items = banners.filter(
    (banner) => banner.url && !failed.includes(banner.url),
  );
  const index = Math.min(active, items.length - 1);
  if (!items.length) return null;
  return (
    <section className="banner-carousel" aria-label={t("ui.promotions")}>
      <div className="banner-carousel__viewport">
        <div
          className="banner-carousel__track"
          style={{ transform: `translateX(${-index * 100}%)` }}
        >
          {items.map((banner, i) => {
            const Tag = banner.jump_url ? "a" : "div";
            return (
              <Tag
                key={banner.url}
                className="banner-carousel__slide"
                href={banner.jump_url || undefined}
                tabIndex={i === index && banner.jump_url ? 0 : -1}
                aria-hidden={i !== index || undefined}
                aria-label={t("ui.openPromotion", { number: i + 1 })}
                onClick={
                  banner.jump_url
                    ? (event) => {
                        event.preventDefault();
                        openUrl(banner.jump_url).catch(console.error);
                      }
                    : undefined
                }
              >
                <img
                  src={banner.url}
                  alt=""
                  draggable={false}
                  onError={() =>
                    setFailed((previous) => [...previous, banner.url])
                  }
                />
              </Tag>
            );
          })}
        </div>
      </div>
      {items.length > 1 && (
        <>
          <div className="banner-carousel__controls">
            <button
              aria-label={t("ui.previousPromotion")}
              onClick={() =>
                setActive((index - 1 + items.length) % items.length)
              }
            >
              <Icon name="back" size={16} />
            </button>
            <button
              aria-label={t("ui.nextPromotion")}
              onClick={() => setActive((index + 1) % items.length)}
            >
              <Icon name="arrow" size={16} />
            </button>
          </div>
          <div className="banner-carousel__indicators" aria-hidden="true">
            {items.map((item, i) => (
              <span key={item.url} className={i === index ? "active" : ""} />
            ))}
          </div>
        </>
      )}
    </section>
  );
}
