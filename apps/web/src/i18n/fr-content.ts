// Long-form legal texts (French reference). Bracketed parts are placeholders the operator
// must complete; the application does not invent a legal entity.
export const frContent = {
  privacy: {
    title: 'Politique de confidentialité',
    sections: [
      {
        heading: 'Responsable du traitement',
        paragraphs: ['[À compléter : raison sociale, adresse et contact du responsable du traitement.]'],
      },
      {
        heading: 'Données traitées',
        paragraphs: [
          'Adresse e-mail ; mot de passe, conservé uniquement sous forme de hachage PBKDF2 ; jetons de session, conservés sous forme de hachage ; clés publiques des machines liées ; identifiants et libellés de vos calculs.',
        ],
      },
      {
        heading: 'Finalités et bases légales',
        paragraphs: [
          'Gérer votre compte et vos machines : exécution du contrat (article 6, paragraphe 1, point b, du RGPD). Sécurité du service et prévention des abus : intérêt légitime (article 6, paragraphe 1, point f).',
        ],
      },
      {
        heading: 'Cookies et traceurs',
        paragraphs: [
          'Ce site n’utilise ni cookie, ni traceur publicitaire, ni outil de mesure d’audience. Le jeton de session est conservé dans le stockage de session de votre navigateur et disparaît à la fermeture de l’onglet. La langue choisie est conservée localement.',
        ],
      },
      {
        heading: 'Destinataires et transferts',
        paragraphs: [
          'Les données sont conservées sur le serveur qui héberge le service. [À compléter : hébergeur réel et localisation des données.]',
        ],
      },
      {
        heading: 'Durée de conservation',
        paragraphs: [
          'Les données du compte sont conservées tant que le compte existe. Les sessions expirent après 24 heures par défaut. Les codes de liaison expirent après 10 minutes.',
        ],
      },
      {
        heading: 'Vos droits',
        paragraphs: [
          'Vous pouvez accéder à vos données, les exporter (fichier JSON depuis la page Compte), les supprimer en supprimant votre compte, et vous opposer à un traitement. Vous pouvez introduire une réclamation auprès de l’autorité de contrôle de votre pays.',
        ],
      },
      {
        heading: 'Limite',
        paragraphs: [
          'Les événements signés publiés sur le réseau pair à pair ne sont pas sous le contrôle de Silicium et ne peuvent pas être effacés par lui. Ils ne contiennent pas de nom ; leur effacement complet n’est toutefois pas garanti.',
        ],
      },
    ],
  },
  legal: {
    title: 'Mentions légales',
    sections: [
      {
        heading: 'Éditeur',
        paragraphs: ['[À compléter : forme juridique, raison sociale, adresse, numéro d’immatriculation, directeur de la publication.]'],
      },
      {
        heading: 'Hébergement',
        paragraphs: ['[À compléter : nom, adresse et contact de l’hébergeur.]'],
      },
      {
        heading: 'Propriété intellectuelle',
        paragraphs: ['[À compléter : licence du logiciel et des contenus.]'],
      },
    ],
  },
  accessibility: {
    title: 'Déclaration d’accessibilité',
    sections: [
      {
        heading: 'État de conformité',
        paragraphs: [
          'Cette application vise la conformité au référentiel WCAG 2.2 niveau AA et à la norme EN 301 549. Un auto-contrôle a été réalisé. Aucun audit par un tiers n’a encore été fait : la conformité n’est pas attestée à ce stade.',
        ],
      },
      {
        heading: 'Contenus non accessibles',
        paragraphs: [
          'Les graphiques de l’application de bureau ne sont pas encore décrits par un texte équivalent. [À compléter après audit.]',
        ],
      },
      {
        heading: 'Technologies et utilisation',
        paragraphs: [
          'Le site est conçu pour un usage au clavier et avec un lecteur d’écran, et pour un zoom élevé. Ces points n’ont été vérifiés qu’en partie (clavier et contrastes sur les pages principales ; zoom à 400 % et largeur de 320 px non encore testés). Les animations sont désactivées si votre système demande de réduire les mouvements.',
        ],
      },
      {
        heading: 'Signaler un problème',
        paragraphs: [
          'Signalez un problème d’accessibilité à [adresse de contact à compléter]. Sans réponse dans un délai raisonnable, vous pouvez saisir l’autorité de contrôle compétente de votre pays.',
        ],
      },
      {
        heading: 'Date',
        paragraphs: ['[Date de publication à compléter.]'],
      },
    ],
  },
}
