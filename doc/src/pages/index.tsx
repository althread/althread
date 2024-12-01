import clsx from 'clsx';
import Link from '@docusaurus/Link';
import useDocusaurusContext from '@docusaurus/useDocusaurusContext';
import Layout from '@theme/Layout';
import HomepageFeatures from '@site/src/components/HomepageFeatures';
import Heading from '@theme/Heading';

import styles from './index.module.css';
import Head from '@docusaurus/Head';

function HomepageHeader() {
  const { siteConfig } = useDocusaurusContext();
  return (
    <header className={clsx('hero hero--primary', styles.heroBanner)}>
      <div className="container">
        <Heading as="h1" className="hero__title">
          {siteConfig.title}
        </Heading>
        <p className="hero__subtitle">{siteConfig.tagline}</p>
        <div className={styles.buttons}>
          <Link
            className="button button--secondary button--lg"
            to="/docs/guide/intro">
            Althread Tutorial - 5min ⏱️
          </Link>
        </div>
      </div>
    </header>
  );
}

export default function Home(): JSX.Element {
  const { siteConfig } = useDocusaurusContext();
  return (
    <div className={styles.landing}>
      <Head>
        <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;600;700&display=swap" rel="stylesheet" />
      </Head>
      <h1>Welcome to Althread</h1>
        <p>A programming language for testing concurrent programs, powered by Rust.</p>
        <div className={styles.buttons}>
          <Link to="/docs/guide/intro" className={styles.button}>Documentation</Link>
          <a href="/editor/" className={styles.button}>Editor</a>
        </div>
        <footer>
          <p>Made with <span style={{color: '#ef4444'}}>♥</span> in Rust. Check it out on <a href="https://github.com/althread/althread" target="_blank">GitHub</a>.</p>
        </footer>
    </div>
  );
}
