import React from 'react';
import { render, screen } from '@testing-library/react';
import '@testing-library/jest-dom';
import { Card, CardHeader, CardTitle, CardBody, CardFooter } from '../Card';

describe('Card', () => {
  // -------------------------------------------------------------------------
  // Rendering
  // -------------------------------------------------------------------------

  it('renders children', () => {
    render(<Card>Card content</Card>);
    expect(screen.getByText('Card content')).toBeInTheDocument();
  });

  it('applies the base ui-card class', () => {
    const { container } = render(<Card />);
    expect(container.firstChild).toHaveClass('ui-card');
  });

  it('merges a custom className', () => {
    const { container } = render(<Card className="my-card" />);
    expect(container.firstChild).toHaveClass('ui-card');
    expect(container.firstChild).toHaveClass('my-card');
  });

  it('forwards extra HTML attributes', () => {
    render(<Card data-testid="card-root" role="region" aria-label="Market" />);
    const el = screen.getByTestId('card-root');
    expect(el).toHaveAttribute('role', 'region');
    expect(el).toHaveAttribute('aria-label', 'Market');
  });

  // -------------------------------------------------------------------------
  // Variants
  // -------------------------------------------------------------------------

  it('does not apply interactive class by default', () => {
    const { container } = render(<Card />);
    expect(container.firstChild).not.toHaveClass('ui-card--interactive');
  });

  it('applies interactive class when interactive=true', () => {
    const { container } = render(<Card interactive />);
    expect(container.firstChild).toHaveClass('ui-card--interactive');
  });

  it('does not apply no-padding class by default', () => {
    const { container } = render(<Card />);
    expect(container.firstChild).not.toHaveClass('ui-card--no-padding');
  });

  it('applies no-padding class when noPadding=true', () => {
    const { container } = render(<Card noPadding />);
    expect(container.firstChild).toHaveClass('ui-card--no-padding');
  });

  it('can combine interactive and noPadding', () => {
    const { container } = render(<Card interactive noPadding />);
    expect(container.firstChild).toHaveClass('ui-card--interactive');
    expect(container.firstChild).toHaveClass('ui-card--no-padding');
  });

  // -------------------------------------------------------------------------
  // Ref forwarding
  // -------------------------------------------------------------------------

  it('forwards ref to the underlying <div>', () => {
    const ref = React.createRef<HTMLDivElement>();
    render(<Card ref={ref} />);
    expect(ref.current).toBeInstanceOf(HTMLDivElement);
  });
});

describe('CardHeader', () => {
  it('renders children and applies the header class', () => {
    const { container } = render(<CardHeader>Header</CardHeader>);
    expect(screen.getByText('Header')).toBeInTheDocument();
    expect(container.firstChild).toHaveClass('ui-card__header');
  });

  it('merges a custom className', () => {
    const { container } = render(<CardHeader className="extra">H</CardHeader>);
    expect(container.firstChild).toHaveClass('ui-card__header');
    expect(container.firstChild).toHaveClass('extra');
  });
});

describe('CardTitle', () => {
  it('renders as <h3> by default', () => {
    render(<CardTitle>My title</CardTitle>);
    expect(screen.getByRole('heading', { level: 3, name: 'My title' })).toBeInTheDocument();
  });

  it.each(['h2', 'h3', 'h4'] as const)('renders as <%s> when as="%s"', (tag) => {
    render(<CardTitle as={tag}>Title</CardTitle>);
    const level = parseInt(tag[1]);
    expect(screen.getByRole('heading', { level, name: 'Title' })).toBeInTheDocument();
  });

  it('applies the title class', () => {
    const { container } = render(<CardTitle>T</CardTitle>);
    expect(container.firstChild).toHaveClass('ui-card__title');
  });
});

describe('CardBody', () => {
  it('renders children and applies the body class', () => {
    const { container } = render(<CardBody>Body content</CardBody>);
    expect(screen.getByText('Body content')).toBeInTheDocument();
    expect(container.firstChild).toHaveClass('ui-card__body');
  });
});

describe('CardFooter', () => {
  it('renders children and applies the footer class', () => {
    const { container } = render(<CardFooter>Footer content</CardFooter>);
    expect(screen.getByText('Footer content')).toBeInTheDocument();
    expect(container.firstChild).toHaveClass('ui-card__footer');
  });
});

describe('Card composition', () => {
  it('renders a full Card with all sub-components', () => {
    render(
      <Card>
        <CardHeader>
          <CardTitle>Market Title</CardTitle>
        </CardHeader>
        <CardBody>Some body text.</CardBody>
        <CardFooter>Actions here</CardFooter>
      </Card>
    );
    expect(screen.getByRole('heading', { name: 'Market Title' })).toBeInTheDocument();
    expect(screen.getByText('Some body text.')).toBeInTheDocument();
    expect(screen.getByText('Actions here')).toBeInTheDocument();
  });
});
