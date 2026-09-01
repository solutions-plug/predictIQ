import React from 'react';

export interface FeatureCardProps {
  icon: string;
  title: string;
  description: string;
  /** When set, the whole card is a link (keyboard-reachable via Tab). */
  href?: string;
}

export const FeatureCard: React.FC<FeatureCardProps> = ({ icon, title, description, href }) => {
  const body = (
    <>
      <img src={icon} alt="" aria-hidden="true" width="64" height="64" />
      <h3>{title}</h3>
      <p>{description}</p>
    </>
  );

  if (href) {
    return (
      <article className="feature-card feature-card--link">
        <a href={href} className="feature-card__link">
          {body}
          <span className="visually-hidden"> — learn more</span>
        </a>
      </article>
    );
  }

  return <article className="feature-card">{body}</article>;
};
