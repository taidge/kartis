export const useBumpSeriesLabels = <
    Datum extends BumpDatum,
    ExtraProps extends BumpSerieExtraProps,
>({
    series,
    position,
    padding,
    color,
    getLabel,
}: {
    series: BumpComputedSerie<Datum, ExtraProps>[]
    position: 'start' | 'end'
    padding: number
    color: InheritedColorConfig<BumpComputedSerie<Datum, ExtraProps>>
    getLabel: Exclude<BumpLabel<Datum, ExtraProps>, false>
}) => {
    const theme = useTheme()
    const getColor = useInheritedColor(color, theme)

    return useMemo(() => {
        let textAnchor: 'start' | 'end'
        let signedPadding: number
        if (position === 'start') {
            textAnchor = 'end'
            signedPadding = padding * -1
        } else {
            textAnchor = 'start'
            signedPadding = padding
        }

        const labels: BumpLabelData<Datum, ExtraProps>[] = []
        series.forEach(serie => {
            let label = serie.id
            if (typeof getLabel === 'function') {
                label = getLabel(serie.data)
            }

            const point =
                position === 'start'
                    ? serie.linePoints[0]
                    : serie.linePoints[serie.linePoints.length - 1]

            // exclude labels for series having missing data at the beginning/end
            if (point?.[0] === null || point?.[1] === null) {
                return
            }

            labels.push({
                id: serie.id,
                label,
                x: point[0] + signedPadding,
                y: point[1],
                color: getColor(serie) as string,
                opacity: serie.opacity,
                serie,
                textAnchor,
            })
        })

        return labels
    }, [series, position, padding, getColor, getLabel])
}
