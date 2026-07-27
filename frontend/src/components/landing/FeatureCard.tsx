import React from 'react';

export interface FeatureCardProps {
  icon: string;
  title: string;
  description: string;
}

export const FeatureCard: React.FC<FeatureCardProps> = ({ icon, title, description }) => (
  <article className="feature-card">
    <img src={icon} alt="" aria-hidden="true" width="64" height="64" />
    <h3>{title}</h3>
    <p>{description}</p>
  </article>
);
