import type { frContent } from './fr-content'

// English legal texts. Same structure as fr-content.ts (enforced by the type).
export const enContent: typeof frContent = {
  privacy: {
    title: 'Privacy policy',
    sections: [
      {
        heading: 'Controller',
        paragraphs: ['[To complete: company name, address and contact of the data controller.]'],
      },
      {
        heading: 'Data processed',
        paragraphs: [
          'E-mail address; password, stored only as a PBKDF2 hash; session tokens, stored as hashes; public keys of linked machines; identifiers and labels of your jobs.',
        ],
      },
      {
        heading: 'Purposes and legal bases',
        paragraphs: [
          'Running your account and machines: performance of a contract (GDPR article 6(1)(b)). Service security and abuse prevention: legitimate interest (article 6(1)(f)).',
        ],
      },
      {
        heading: 'Cookies and trackers',
        paragraphs: [
          'This site uses no cookie, no advertising tracker and no audience measurement tool. The session token is kept in your browser session storage and disappears when the tab closes. The chosen language is stored locally.',
        ],
      },
      {
        heading: 'Recipients and transfers',
        paragraphs: [
          'Data stays on the server that hosts the service. [To complete: actual host and data location.]',
        ],
      },
      {
        heading: 'Retention',
        paragraphs: [
          'Account data is kept while the account exists. Sessions expire after 24 hours by default. Link codes expire after 10 minutes.',
        ],
      },
      {
        heading: 'Your rights',
        paragraphs: [
          'You can access your data, export it (JSON file from the Account page), erase it by deleting your account, and object to processing. You can lodge a complaint with the supervisory authority of your country.',
        ],
      },
      {
        heading: 'Limit',
        paragraphs: [
          'Signed events published on the peer-to-peer network are not under Silicium’s control and cannot be erased by it. They contain no name; complete erasure is nevertheless not guaranteed.',
        ],
      },
    ],
  },
  legal: {
    title: 'Legal notice',
    sections: [
      {
        heading: 'Publisher',
        paragraphs: ['[To complete: legal form, company name, address, registration number, publication director.]'],
      },
      {
        heading: 'Hosting',
        paragraphs: ['[To complete: name, address and contact of the host.]'],
      },
      {
        heading: 'Intellectual property',
        paragraphs: ['[To complete: licence of the software and of the content.]'],
      },
    ],
  },
  accessibility: {
    title: 'Accessibility statement',
    sections: [
      {
        heading: 'Conformance status',
        paragraphs: [
          'This application aims to conform to WCAG 2.2 level AA and to EN 301 549. A self-check has been carried out. No third-party audit has been done yet: conformance is not attested at this stage.',
        ],
      },
      {
        heading: 'Content not accessible',
        paragraphs: [
          'Charts in the desktop application do not yet have an equivalent text description. [To complete after audit.]',
        ],
      },
      {
        heading: 'Technologies and use',
        paragraphs: [
          'The site is designed for keyboard and screen reader use, and for high zoom. These points were only partly verified (keyboard and contrast on the main pages; 400% zoom and 320 px width not yet tested). Animations are disabled when your system requests reduced motion.',
        ],
      },
      {
        heading: 'Reporting a problem',
        paragraphs: [
          'Report an accessibility problem to [contact address to complete]. Without a reply within a reasonable time, you can refer the matter to the competent supervisory authority of your country.',
        ],
      },
      {
        heading: 'Date',
        paragraphs: ['[Publication date to complete.]'],
      },
    ],
  },
}
