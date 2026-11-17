// Generated from the Git-exact original Nivo function; source range is recorded.
import {area,line,curveBasis,curveLinear} from 'd3-shape';
export const computeShapeGenerators =                        (
    interpolation                                       ,
    direction                                   
)                                               => {
    // area generator which is used to draw funnel chart parts
    const areaGenerator                      = area                 ()
    if (direction === 'vertical') {
        areaGenerator
            .curve(interpolation === 'smooth' ? curveBasis : curveLinear)
            .x0(d => d.x0)
            .x1(d => d.x1)
            .y(d => d.y)
    } else {
        areaGenerator
            .curve(interpolation === 'smooth' ? curveBasis : curveLinear)
            .y0(d => d.y0)
            .y1(d => d.y1)
            .x(d => d.x)
    }

    return [
        areaGenerator,
        // we're using a different line generator to draw borders, this way
        // we we don't have borders joining each side of the parts.
        // it's important to have an empty point when defining the points
        // to be used along with this, otherwise we'll get a line between both sides.
        line                 ()
            .defined(d => d !== null)
            .x(d => d .x)
            .y(d => d .y)
            .curve(interpolation === 'smooth' ? curveBasis : curveLinear),
    ]
};
