<?xml version="1.0" encoding="UTF-8"?>
<xsl:stylesheet version="1.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform">
  <xsl:output method="html" indent="yes" encoding="UTF-8"  version="4.0" />
  <xsl:param name="version" />
  <xsl:param name="assetRoot" />
  <xsl:param name="apiRoot" />
  <xsl:param name="loginURL" />
  <xsl:param name="logoutURL" />
  <xsl:param name="uiRoot" />
  <xsl:template match="page">
    <html>
      <head>
        <title>milton</title>
        <link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.2.0/css/all.min.css" />
        <link rel="stylesheet" href="{$assetRoot}main.css?v={$version}" />
        <meta name="version" content="{$version}" />
        <meta name="apiRoot" content="{$apiRoot}" />
        <meta name="uiRoot" content="{$uiRoot}" />
        <meta name="loginURL" content="{$loginURL}" />
        <meta name="logoutURL" content="{$logoutURL}" />
        <meta name="viewport" content="width=device-width, initial-scale=1, maximum-scale=1, minimum-scale=1" />
      </head>
      <body>
        <script type="text/javascript" src="{$assetRoot}main.js?v={$version}"></script>
        <script type="text/javascript" src="{$assetRoot}boot.js?v={$version}"></script>
      </body>
    </html>
  </xsl:template>
</xsl:stylesheet>
