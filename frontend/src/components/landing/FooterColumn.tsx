import React from 'react';

export interface FooterColumnLink {
  href: string;
  label: string;
  /** External links open in a new tab and get rel="noopener". */
  external?: boolean;
}

export interface FooterColumnProps {
  heading: string;
  headingLevel?: 'h2' | 'h3';
  tagline?: string;
  links?: FooterColumnLink[];
  /** A newsletter-signup embed (or any node) rendered under the heading. */
  children?: React.ReactNode;
}

export const FooterColumn: React.FC<FooterColumnProps> = ({
  heading,
  headingLevel = 'h3',
  tagline,
  links,
  children,
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
              <a
                href={link.href}
                {...(link.external
                  ? { target: '_blank', rel: 'noopener noreferrer' }
                  : {})}
              >
                {link.label}
              </a>
            </li>
          ))}
        </ul>
      )}
      {children}
    </div>
  );
};
