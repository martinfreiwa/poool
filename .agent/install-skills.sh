#!/bin/bash
set -e
mkdir -p /Users/martin/Projects/poool/.agent/skills
cd /Users/martin/Projects/poool/.agent/skills

echo "Installing anthropics/skills..."
git clone https://github.com/anthropics/skills.git anthropics_temp
mv anthropics_temp/skills/* . || true
rm -rf anthropics_temp

echo "Installing obra/superpowers..."
git clone https://github.com/obra/superpowers.git superpowers_temp
mv superpowers_temp/skills/* . || true
rm -rf superpowers_temp

echo "Installing obra/superpowers-lab..."
git clone https://github.com/obra/superpowers-lab.git superpowers_lab_temp
mv superpowers_lab_temp/skills/* . || true
rm -rf superpowers_lab_temp

echo "Installing conorluddy/ios-simulator-skill..."
git clone https://github.com/conorluddy/ios-simulator-skill.git

echo "Installing jthack/ffuf_claude_skill..."
git clone https://github.com/jthack/ffuf_claude_skill.git

echo "Installing lackeyjb/playwright-skill..."
git clone https://github.com/lackeyjb/playwright-skill.git

echo "Installing chrisvoncsefalvay/claude-d3js-skill..."
git clone https://github.com/chrisvoncsefalvay/claude-d3js-skill.git

echo "Installing K-Dense-AI/claude-scientific-skills..."
git clone https://github.com/K-Dense-AI/claude-scientific-skills.git
mv claude-scientific-skills/skills/* . || true
rm -rf claude-scientific-skills

echo "Installing alonw0/web-asset-generator..."
git clone https://github.com/alonw0/web-asset-generator.git

echo "Installing asklokesh/claudeskill-loki-mode..."
git clone https://github.com/asklokesh/claudeskill-loki-mode.git

echo "Installing trailofbits/skills..."
git clone https://github.com/trailofbits/skills.git trailofbits_temp
mv trailofbits_temp/skills/* . || true
rm -rf trailofbits_temp

echo "Installing zarazhangrui/frontend-slides..."
git clone https://github.com/zarazhangrui/frontend-slides.git frontend-slides-temp
mv frontend-slides-temp/skills/* . || true
rm -rf frontend-slides-temp

echo "Installing expo/skills..."
git clone https://github.com/expo/skills.git expo_temp
mv expo_temp/skills/* . || true
rm -rf expo_temp

echo "Done."
