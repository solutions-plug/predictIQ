import React from 'react';

export interface FooterColumnLink {
  href: string;
  label: string;
}

export interface FooterColumnProps {
  heading: string;
  headingLevel?: 'h2' | 'h3';
  tagline?: string;
  links?: FooterColumnLink[];
}

export const FooterColumn: React.FC<FooterColumnProps> = ({
  heading,
  headingLevel = 'h3',
  tagline,
  links,
}) => {
  const Heading = headingLevel;
  return (
    <div className="footer-section">
      <Heading>{heading}</Heading>
      {tagline && <p>{tagline}</p>}
      {links && (
        <ul>
          {links.map((link) => (
            <li key={link.href}>
              <a href={link.href}>{link.label}</a>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
};
