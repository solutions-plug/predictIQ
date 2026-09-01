import React from 'react';

export interface StepProps {
  title: string;
  description: string;
  /**
   * Link to the real feature this step describes. The step number itself is
   * derived by CSS `counter()` from the `<ol>` order, never passed in - so it
   * can't drift from the visual list.
   */
  href?: string;
}

export const Step: React.FC<StepProps> = ({ title, description, href }) => (
  <li className="step">
    <h3 className="step__title">
      {href ? (
        <a href={href} className="step__link">
          {title}
        </a>
      ) : (
        title
      )}
    </h3>
    <p className="step__description">{description}</p>
  </li>
);
